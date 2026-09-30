//! Caps on how many requests the exit works on at once.
//!
//! A global cap bounds memory and upstream load; a per-client cap (keyed by
//! the anonymous reply tag) stops one client from taking every slot.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

/// Requests in flight across all clients. With responses capped at
/// `policy::MAX_RESPONSE_BYTES` (4 MiB), this bounds response buffers to
/// ~128 MB, within the planned 256 MB `MemoryMax`.
pub const MAX_IN_FLIGHT: usize = 32;

/// Requests in flight for one client (one anonymous sender tag). A local
/// proxy is a single Nym client, so this is also one user's burst limit.
pub const MAX_IN_FLIGHT_PER_CLIENT: usize = 16;

/// Tracks in-flight requests, globally and per client.
pub struct Limiter<K> {
    max_total: usize,
    max_per_client: usize,
    state: Mutex<State<K>>,
}

struct State<K> {
    total: usize,
    per_client: HashMap<K, usize>,
}

/// A slot held for one request; released when dropped.
pub struct Permit<K: Eq + Hash + Clone> {
    limiter: Arc<Limiter<K>>,
    client: K,
}

impl<K: Eq + Hash + Clone> Limiter<K> {
    pub fn new(max_total: usize, max_per_client: usize) -> Arc<Self> {
        Arc::new(Self {
            max_total,
            max_per_client,
            state: Mutex::new(State {
                total: 0,
                per_client: HashMap::new(),
            }),
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

impl<K: Eq + Hash + Clone> Drop for Permit<K> {
    fn drop(&mut self) {
        let mut state = self.limiter.state.lock().unwrap_or_else(|p| p.into_inner());
        state.total -= 1;
        if let Some(count) = state.per_client.get_mut(&self.client) {
            *count -= 1;
            if *count == 0 {
                state.per_client.remove(&self.client);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn per_client_cap() {
        let limiter = Limiter::new(10, 2);
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
        let limiter = Limiter::new(3, 3);
        let held: Vec<_> = ["a", "b", "c"]
            .iter()
            .map(|c| limiter.try_acquire(*c).unwrap())
            .collect();
        assert!(limiter.try_acquire("d").is_none());
        drop(held);
        assert_eq!(limiter.in_flight(), 0);
        assert!(limiter.try_acquire("d").is_some());
    }
}
