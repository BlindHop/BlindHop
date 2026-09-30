//! Mock-based unit tests for the BlindHop proxy and common crates.
//!
//! These tests are fast, offline, and CI-safe — they don't require
//! Nym connectivity or a running exit service.
//!
//! Run: `cargo test -p blindhop-common --test proxy_unit`

use blindhop_common::config::{BlindHopConfig, ExitBackendType, PrivacyMode};
use blindhop_common::metrics::MetricsCollector;
use blindhop_common::rpc::{JsonRpcRequest, JsonRpcResponse, MessageType, MixnetMessage};

/// Test that the default config has sensible defaults.
#[test]
fn test_default_config() {
    let config = BlindHopConfig::default();
    assert!(matches!(config.privacy_mode, PrivacyMode::Full));
    assert!(matches!(
        config.exit_backend,
        ExitBackendType::ServiceProvider
    ));
}

/// Test that privacy mode has correct properties.
#[test]
fn test_privacy_mode_properties() {
    assert_eq!(PrivacyMode::None.hop_count(), 0);
    assert!(!PrivacyMode::None.has_cover_traffic());

    assert_eq!(PrivacyMode::Fast.hop_count(), 2);
    assert!(!PrivacyMode::Fast.has_cover_traffic());

    assert_eq!(PrivacyMode::Full.hop_count(), 5);
    assert!(PrivacyMode::Full.has_cover_traffic());
}

/// Test that JSON-RPC request round-trips through serialization correctly.
#[test]
fn test_rpc_request_roundtrip() {
    let request = JsonRpcRequest::new(42, "chain_getHeader", vec![]);

    let json = serde_json::to_vec(&request).unwrap();
    let decoded: JsonRpcRequest = serde_json::from_slice(&json).unwrap();

    assert_eq!(decoded.method, "chain_getHeader");
    assert_eq!(decoded.id, 42);
}

/// Test that JSON-RPC response parsing handles success and error cases.
#[test]
fn test_rpc_response_parsing() {
    let success_json = r#"{"jsonrpc":"2.0","id":1,"result":"0xabcd"}"#;
    let response: JsonRpcResponse = serde_json::from_str(success_json).unwrap();
    assert!(response.result.is_some());
    assert!(response.error.is_none());

    let error_json =
        r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#;
    let response: JsonRpcResponse = serde_json::from_str(error_json).unwrap();
    assert!(response.result.is_none());
    assert!(response.error.is_some());
}

/// Test MixnetMessage envelope wrapping and unwrapping.
#[test]
fn test_mixnet_message_envelope() {
    let payload = br#"{"jsonrpc":"2.0","id":1,"method":"chain_getHeader"}"#;

    let msg = MixnetMessage {
        msg_type: MessageType::Request,
        payload: payload.to_vec(),
        correlation_id: 1,
        accepts_compression: false,
    };

    let encoded = serde_json::to_vec(&msg).unwrap();
    let decoded: MixnetMessage = serde_json::from_slice(&encoded).unwrap();

    assert!(matches!(decoded.msg_type, MessageType::Request));
    assert_eq!(decoded.payload, payload.to_vec());
}

/// Test MixnetMessage with response type.
#[test]
fn test_mixnet_message_response() {
    let payload = br#"{"jsonrpc":"2.0","id":1,"result":"0x1234"}"#;

    let msg = MixnetMessage {
        msg_type: MessageType::Response,
        payload: payload.to_vec(),
        correlation_id: 1,
        accepts_compression: false,
    };

    let encoded = serde_json::to_vec(&msg).unwrap();
    let decoded: MixnetMessage = serde_json::from_slice(&encoded).unwrap();

    assert!(matches!(decoded.msg_type, MessageType::Response));
}

/// Test metrics collector latency recording and snapshot.
#[test]
fn test_metrics_snapshot() {
    let collector = MetricsCollector::new(100);

    // Record 100 latencies from 1.0 to 100.0 ms
    for i in 1..=100 {
        collector.record_latency(i as f64);
    }

    let snapshot = collector.snapshot();

    // p50 should be around 50ms
    assert!(
        snapshot.latency_p50_ms >= 45.0 && snapshot.latency_p50_ms <= 55.0,
        "p50 was {}ms",
        snapshot.latency_p50_ms
    );

    // p95 should be around 95ms
    assert!(
        snapshot.latency_p95_ms >= 90.0 && snapshot.latency_p95_ms <= 100.0,
        "p95 was {}ms",
        snapshot.latency_p95_ms
    );
}

/// Test metrics collector eviction when capacity is exceeded.
#[test]
fn test_metrics_eviction() {
    let collector = MetricsCollector::new(10);

    // Record 20 latencies — oldest should be evicted
    for i in 1..=20 {
        collector.record_latency(i as f64);
    }

    // Only the last 10 should remain (11-20ms)
    let snapshot = collector.snapshot();
    assert!(
        snapshot.latency_p50_ms >= 14.0 && snapshot.latency_p50_ms <= 17.0,
        "p50 was {}ms",
        snapshot.latency_p50_ms
    );
}

/// Test metrics reset clears all data.
#[test]
fn test_metrics_reset() {
    let collector = MetricsCollector::new(100);

    collector.record_latency(100.0);
    let snapshot = collector.snapshot();
    assert!(snapshot.latency_p50_ms > 0.0);

    collector.reset();
    let snapshot = collector.snapshot();
    // After reset, percentile of empty data returns 0.0
    assert_eq!(snapshot.latency_p50_ms, 0.0);
}

/// Test privacy mode serialization/deserialization for control messages.
#[test]
fn test_privacy_mode_serde() {
    let mode = PrivacyMode::Full;
    let json = serde_json::to_string(&mode).unwrap();
    let decoded: PrivacyMode = serde_json::from_str(&json).unwrap();
    assert!(matches!(decoded, PrivacyMode::Full));

    let mode = PrivacyMode::None;
    let json = serde_json::to_string(&mode).unwrap();
    let decoded: PrivacyMode = serde_json::from_str(&json).unwrap();
    assert!(matches!(decoded, PrivacyMode::None));
}

/// Test control message parsing (blindhop_setPrivacyMode).
#[test]
fn test_control_message_parsing() {
    let control_msg =
        r#"{"jsonrpc":"2.0","id":99,"method":"blindhop_setPrivacyMode","params":["fast"]}"#;
    let request: JsonRpcRequest = serde_json::from_str(control_msg).unwrap();

    assert_eq!(request.method, "blindhop_setPrivacyMode");
    assert!(request.method.starts_with("blindhop_"));

    let mode_str = request.params[0].as_str().unwrap();
    assert_eq!(mode_str, "fast");
}

/// Test metrics query control message.
#[test]
fn test_metrics_control_message() {
    let control_msg = r#"{"jsonrpc":"2.0","id":100,"method":"blindhop_getMetrics","params":[]}"#;
    let request: JsonRpcRequest = serde_json::from_str(control_msg).unwrap();

    assert_eq!(request.method, "blindhop_getMetrics");
    assert!(request.method.starts_with("blindhop_"));
}
