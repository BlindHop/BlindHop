//! Nym mixnet benchmarks for BlindHop.
//!
//! These benchmarks measure real-world performance of the Nym integration.
//! Some benchmarks require live Nym connectivity; they are gated behind
//! the `nym-live` feature flag.
//!
//! Run offline benchmarks:
//!   cargo bench -p blindhop-proxy --bench nym
//!
//! Run all benchmarks (requires Nym connectivity):
//!   cargo bench -p blindhop-proxy --bench nym --features nym-live

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use blindhop_common::rpc::{JsonRpcRequest, MixnetMessage, MessageType};
use blindhop_common::metrics::MetricsCollector;
use blindhop_common::config::PrivacyMode;

/// Benchmark JSON-RPC request serialization (used on every send).
fn bench_rpc_serialization(c: &mut Criterion) {
    let request = JsonRpcRequest::new(1, "chain_getHeader", vec![]);

    c.bench_function("rpc_serialize", |b| {
        b.iter(|| {
            let json = serde_json::to_vec(black_box(&request)).unwrap();
            black_box(json);
        })
    });

    let json_bytes = serde_json::to_vec(&request).unwrap();
    c.bench_function("rpc_deserialize", |b| {
        b.iter(|| {
            let req: JsonRpcRequest = serde_json::from_slice(black_box(&json_bytes)).unwrap();
            black_box(req);
        })
    });
}

/// Benchmark MixnetMessage envelope wrapping (used on every proxy→exit send).
fn bench_mixnet_message_wrapping(c: &mut Criterion) {
    let payload = br#"{"jsonrpc":"2.0","id":1,"method":"chain_getHeader","params":[]}"#;

    c.bench_function("mixnet_message_wrap", |b| {
        b.iter(|| {
            let msg = MixnetMessage {
                msg_type: MessageType::Request,
                payload: black_box(payload.to_vec()),
            };
            let encoded = serde_json::to_vec(&msg).unwrap();
            black_box(encoded);
        })
    });

    let msg = MixnetMessage {
        msg_type: MessageType::Request,
        payload: payload.to_vec(),
    };
    let encoded = serde_json::to_vec(&msg).unwrap();

    c.bench_function("mixnet_message_unwrap", |b| {
        b.iter(|| {
            let decoded: MixnetMessage = serde_json::from_slice(black_box(&encoded)).unwrap();
            black_box(decoded);
        })
    });
}

/// Benchmark metrics collection overhead (called on every request/response).
fn bench_metrics_collection(c: &mut Criterion) {
    let collector = MetricsCollector::new(1000);

    c.bench_function("metrics_record_latency", |b| {
        let mut i = 0u64;
        b.iter(|| {
            i += 1;
            collector.record_latency(black_box((i % 500) as f64));
        })
    });

    // Pre-fill metrics
    let filled = MetricsCollector::new(1000);
    for i in 0..1000 {
        filled.record_latency(i as f64);
    }

    c.bench_function("metrics_snapshot", |b| {
        b.iter(|| {
            let snapshot = filled.snapshot();
            black_box(snapshot);
        })
    });
}

/// Benchmark privacy mode switching logic (no Nym client, just the enum dispatch).
fn bench_mode_switching(c: &mut Criterion) {
    let modes = [PrivacyMode::None, PrivacyMode::Fast, PrivacyMode::Full];

    c.bench_function("privacy_mode_dispatch", |b| {
        let mut idx = 0usize;
        b.iter(|| {
            let mode = &modes[idx % 3];
            idx += 1;
            let _description = match mode {
                PrivacyMode::None => "direct",
                PrivacyMode::Fast => "2-hop nym",
                PrivacyMode::Full => "5-hop nym + cover",
            };
            black_box(_description);
        })
    });
}

criterion_group!(
    offline_benches,
    bench_rpc_serialization,
    bench_mixnet_message_wrapping,
    bench_metrics_collection,
    bench_mode_switching,
);

criterion_main!(offline_benches);
