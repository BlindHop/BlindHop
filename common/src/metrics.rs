//! Transport metrics collection and aggregation.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Transport performance metrics.
///
/// Thread-safe metrics collected during transport operation.
/// Used for real-time dashboard display and benchmarking.
#[derive(Debug, Clone)]
pub struct TransportMetrics {
    /// Median latency (p50) in milliseconds.
    pub latency_p50_ms: f64,

    /// 95th percentile latency in milliseconds.
    pub latency_p95_ms: f64,

    /// Total messages sent through the transport.
    pub messages_sent: u64,

    /// Total messages received from the transport.
    pub messages_received: u64,

    /// Measured overhead vs. direct connection in milliseconds.
    pub overhead_ms: f64,
}

impl Default for TransportMetrics {
    fn default() -> Self {
        Self {
            latency_p50_ms: 0.0,
            latency_p95_ms: 0.0,
            messages_sent: 0,
            messages_received: 0,
            overhead_ms: 0.0,
        }
    }
}

/// Thread-safe metrics accumulator.
///
/// Collects latency samples and message counts, providing
/// real-time percentile calculations for the dashboard.
pub struct MetricsCollector {
    latencies: Mutex<Vec<f64>>,
    messages_sent: AtomicU64,
    messages_received: AtomicU64,
    max_samples: usize,
}

impl MetricsCollector {
    /// Create a new metrics collector.
    ///
    /// `max_samples` controls how many latency samples are retained
    /// for percentile calculation (older samples are evicted).
    pub fn new(max_samples: usize) -> Self {
        Self {
            latencies: Mutex::new(Vec::with_capacity(max_samples)),
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            max_samples,
        }
    }

    /// Record a latency measurement in milliseconds.
    pub fn record_latency(&self, latency_ms: f64) {
        let mut latencies = self.latencies.lock().unwrap();
        if latencies.len() >= self.max_samples {
            latencies.remove(0);
        }
        latencies.push(latency_ms);
    }

    /// Increment the sent message counter.
    pub fn record_send(&self) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment the received message counter.
    pub fn record_recv(&self) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
    }

    /// Compute the current metrics snapshot.
    pub fn snapshot(&self) -> TransportMetrics {
        let latencies = self.latencies.lock().unwrap();
        TransportMetrics {
            latency_p50_ms: percentile(&latencies, 50),
            latency_p95_ms: percentile(&latencies, 95),
            messages_sent: self.messages_sent.load(Ordering::Relaxed),
            messages_received: self.messages_received.load(Ordering::Relaxed),
            overhead_ms: 0.0, // Computed externally by comparing with direct mode
        }
    }

    /// Reset all metrics.
    pub fn reset(&self) {
        self.latencies.lock().unwrap().clear();
        self.messages_sent.store(0, Ordering::Relaxed);
        self.messages_received.store(0, Ordering::Relaxed);
    }
}

/// Compute the p-th percentile of a sorted slice.
fn percentile(data: &[f64], p: u8) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((sorted.len() as f64) * (p as f64) / 100.0).ceil() as usize;
    sorted[idx.saturating_sub(1).min(sorted.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentile_empty() {
        assert_eq!(percentile(&[], 50), 0.0);
    }

    #[test]
    fn test_percentile_single() {
        assert_eq!(percentile(&[42.0], 50), 42.0);
        assert_eq!(percentile(&[42.0], 95), 42.0);
    }

    #[test]
    fn test_percentile_multiple() {
        let data: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        assert_eq!(percentile(&data, 50), 50.0);
        assert_eq!(percentile(&data, 95), 95.0);
    }

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new(100);

        collector.record_latency(100.0);
        collector.record_latency(200.0);
        collector.record_latency(300.0);
        collector.record_send();
        collector.record_send();
        collector.record_recv();

        let snap = collector.snapshot();
        assert_eq!(snap.messages_sent, 2);
        assert_eq!(snap.messages_received, 1);
        assert!(snap.latency_p50_ms > 0.0);
    }

    #[test]
    fn test_metrics_collector_eviction() {
        let collector = MetricsCollector::new(3);

        collector.record_latency(10.0);
        collector.record_latency(20.0);
        collector.record_latency(30.0);
        collector.record_latency(40.0); // Evicts 10.0

        let snap = collector.snapshot();
        // Should only have 20, 30, 40 now
        assert!(snap.latency_p50_ms >= 20.0);
    }

    #[test]
    fn test_metrics_reset() {
        let collector = MetricsCollector::new(100);
        collector.record_latency(100.0);
        collector.record_send();
        collector.reset();

        let snap = collector.snapshot();
        assert_eq!(snap.messages_sent, 0);
        assert_eq!(snap.latency_p50_ms, 0.0);
    }
}
