//! Caps on how many requests the exit works on at once.
//!
//! A global cap bounds memory and upstream load; a per-client cap (keyed by
//! the anonymous reply tag) stops one client from taking every slot. A
//! request that finds no free slot may wait briefly for one, so short bursts
//! (e.g. while upstream connections are still being opened) aren't refused.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::Notify;

/// Requests in flight across all clients. Measured ~3.9 MB of memory per
/// in-flight 1.2 MB response (~3.2x: upstream buffer, reply, compressed
/// copy, frame); at the 4 MiB response cap that's ~13 MB each, so 16 in
/// flight stays near 210 MB, within the planned 256 MB `MemoryMax`.
pub const MAX_IN_FLIGHT: usize = 16;

/// Requests in flight for one client (one anonymous sender tag), so one
/// client can't take every slot. A local proxy is a single Nym client, so
/// this is also one user's burst limit. A request counts only until its
/// reply is queued, so short requests rarely reach it.
pub const MAX_IN_FLIGHT_PER_CLIENT: usize = 8;

/// Requests allowed to wait for a slot at once, across all clients. Beyond
/// this, requests are refused as busy immediately. Waiting requests hold only
/// their (validated, <= 1 MiB) request payload.
pub const MAX_WAITING: usize = 32;

/// How long a request may wait for a slot before being refused as busy.
pub const MAX_WAIT_FOR_SLOT: Duration = Duration::from_secs(5);

/// Tracks in-flight requests, globally and per client.
pub struct Limiter<K> {
    max_total: usize,
    max_per_client: usize,
    max_waiting: usize,
    state: Mutex<State<K>>,
    /// Woken whenever a slot is released.
    released: Notify,
}

struct State<K> {
    total: usize,
    per_client: HashMap<K, usize>,
    waiting: usize,
}

/// A slot held for one request; released when dropped.
pub struct Permit<K: Eq + Hash + Clone> {
    limiter: Arc<Limiter<K>>,
    client: K,
}

impl<K: Eq + Hash + Clone> Limiter<K> {
    pub fn new(max_total: usize, max_per_client: usize, max_waiting: usize) -> Arc<Self> {
        Arc::new(Self {
            max_total,
            max_per_client,
            max_waiting,
            state: Mutex::new(State {
                total: 0,
                per_client: HashMap::new(),
                waiting: 0,
            }),
            released: Notify::new(),
        })
    }

    /// Take a slot for `client`, waiting up to `max_wait` for one to free up.
    /// Returns `None` if none frees up in time, or if too many requests are
    /// already waiting.
    pub async fn acquire(self: &Arc<Self>, client: K, max_wait: Duration) -> Option<Permit<K>> {
        if let Some(permit) = self.try_acquire(client.clone()) {
            return Some(permit);
        }
        let _waiting = self.start_waiting()?;
        let deadline = tokio::time::Instant::now() + max_wait;
        loop {
            // Register for the wakeup before re-checking, so a release that
            // happens in between isn't missed.
            let released = self.released.notified();
            tokio::pin!(released);
            released.as_mut().enable();
            if let Some(permit) = self.try_acquire(client.clone()) {
                return Some(permit);
            }
            if tokio::time::timeout_at(deadline, released).await.is_err() {
                return self.try_acquire(client);
            }
        }
    }

    fn start_waiting(self: &Arc<Self>) -> Option<WaitingGuard<K>> {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if state.waiting >= self.max_waiting {
            return None;
        }
        state.waiting += 1;
        Some(WaitingGuard {
            limiter: self.clone(),
        })
    }

    /// Take a slot for `client`, or `None` if either cap is reached.
    pub fn try_acquire(self: &Arc<Self>, client: K) -> Option<Permit<K>> {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let client_count = state.per_client.get(&client).copied().unwrap_or(0);
        if state.total >= self.max_total || client_count >= self.max_per_client {
            return None;
        }
        state.total += 1;
        state.per_client.insert(client.clone(), client_count + 1);
        Some(Permit {
            limiter: self.clone(),
            client,
        })
    }

    #[cfg(test)]
    fn in_flight(&self) -> usize {
        self.state.lock().unwrap().total
    }
}

/// Counts a request as waiting for a slot; released when dropped.
struct WaitingGuard<K> {
    limiter: Arc<Limiter<K>>,
}

impl<K> Drop for WaitingGuard<K> {
    fn drop(&mut self) {
        let mut state = self.limiter.state.lock().unwrap_or_else(|p| p.into_inner());
        state.waiting -= 1;
    }
}

impl<K: Eq + Hash + Clone> Drop for Permit<K> {
    fn drop(&mut self) {
        {
            let mut state = self.limiter.state.lock().unwrap_or_else(|p| p.into_inner());
            state.total -= 1;
            if let Some(count) = state.per_client.get_mut(&self.client) {
                *count -= 1;
                if *count == 0 {
                    state.per_client.remove(&self.client);
                }
            }
        }
        // Wake waiting requests; each re-checks whether it can take the slot.
        self.limiter.released.notify_waiters();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn per_client_cap() {
        let limiter = Limiter::new(10, 2, 0);
        let a1 = limiter.try_acquire("a");
        let a2 = limiter.try_acquire("a");
        assert!(a1.is_some() && a2.is_some());
        assert!(limiter.try_acquire("a").is_none(), "third for one client");
        assert!(
            limiter.try_acquire("b").is_some(),
            "other clients unaffected"
        );
        drop(a1);
        assert!(limiter.try_acquire("a").is_some(), "slot freed on drop");
    }

    #[test]
    fn global_cap() {
        let limiter = Limiter::new(3, 3, 0);
        let held: Vec<_> = ["a", "b", "c"]
            .iter()
            .map(|c| limiter.try_acquire(*c).unwrap())
            .collect();
        assert!(limiter.try_acquire("d").is_none());
        drop(held);
        assert_eq!(limiter.in_flight(), 0);
        assert!(limiter.try_acquire("d").is_some());
    }

    #[tokio::test]
    async fn waiting_request_gets_a_released_slot() {
        let limiter = Limiter::new(1, 1, 4);
        let held = limiter.try_acquire("a").unwrap();
        let waiter = {
            let limiter = limiter.clone();
            tokio::spawn(
                async move { limiter.acquire("a", Duration::from_secs(5)).await.is_some() },
            )
        };
        tokio::time::sleep(Duration::from_millis(50)).await;
        drop(held);
        assert!(waiter.await.unwrap(), "waiter should get the freed slot");
    }

    #[tokio::test]
    async fn waiting_times_out_as_busy() {
        let limiter = Limiter::new(1, 1, 4);
        let _held = limiter.try_acquire("a").unwrap();
        let start = std::time::Instant::now();
        assert!(
            limiter
                .acquire("a", Duration::from_millis(100))
                .await
                .is_none()
        );
        assert!(start.elapsed() >= Duration::from_millis(100));
    }

    #[tokio::test]
    async fn waiting_is_bounded() {
        let limiter = Limiter::new(1, 1, 1);
        let _held = limiter.try_acquire("a").unwrap();
        let first = {
            let limiter = limiter.clone();
            tokio::spawn(async move {
                limiter
                    .acquire("b", Duration::from_millis(300))
                    .await
                    .is_some()
            })
        };
        tokio::time::sleep(Duration::from_millis(50)).await;
        // One request is already waiting, so this one is refused at once.
        let start = std::time::Instant::now();
        assert!(limiter.acquire("c", Duration::from_secs(5)).await.is_none());
        assert!(start.elapsed() < Duration::from_millis(100));
        assert!(!first.await.unwrap());
    }
}
