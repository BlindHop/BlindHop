//! Live Nym mainnet integration tests for BlindHop.
//!
//! These tests require:
//! - Active Nym network connectivity
//! - A running `blindhop-exit` service (with its Nym address)
//!
//! They are gated behind the `nym-live` feature flag and marked `#[ignore]`
//! so they don't run in normal CI. Run manually or via nightly CI:
//!
//!   BLINDHOP_EXIT_NYM_ADDR=<address> cargo test --test nym_integration --features nym-live -- --ignored

#[cfg(feature = "nym-live")]
mod live_tests {
    use blindhop_common::config::PrivacyMode;
    use blindhop_common::rpc::{MessageType, MixnetMessage};

    /// Test: Connect a Nym client, send a message, receive a reply.
    /// Requires a running exit service.
    #[tokio::test]
    #[ignore = "Requires live Nym connectivity and running exit service"]
    async fn test_nym_roundtrip() {
        let exit_addr = std::env::var("BLINDHOP_EXIT_NYM_ADDR")
            .expect("Set BLINDHOP_EXIT_NYM_ADDR to exit service's Nym address");

        // This test verifies the fundamental Nym SDK send/receive path
        // 1. Connect a new MixnetClient
        // 2. Send a MixnetMessage (Request) to the exit address
        // 3. Wait for a SURB reply
        // 4. Verify the response is a valid MixnetMessage (Response)

        // Placeholder — actual implementation requires nym-sdk MixnetClient
        eprintln!("Exit address: {}", exit_addr);
        eprintln!("TODO: Implement with actual nym-sdk client");
    }

    /// Test: Send a chain_getHeader request through the full path.
    /// Proxy → Nym → Exit → Substrate → SURB reply → Proxy.
    #[tokio::test]
    #[ignore = "Requires live Nym connectivity and running exit service"]
    async fn test_nym_to_substrate() {
        let exit_addr = std::env::var("BLINDHOP_EXIT_NYM_ADDR")
            .expect("Set BLINDHOP_EXIT_NYM_ADDR to exit service's Nym address");

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "chain_getHeader",
            "params": []
        });

        let msg = MixnetMessage {
            msg_type: MessageType::Request,
            payload: serde_json::to_vec(&request).unwrap(),
            correlation_id: 1,
        };

        let _encoded = serde_json::to_vec(&msg).unwrap();

        eprintln!("Exit address: {}", exit_addr);
        eprintln!("TODO: Send through Nym, verify Substrate response");
    }

    /// Test: Switch privacy modes while connected to Nym.
    #[tokio::test]
    #[ignore = "Requires live Nym connectivity"]
    async fn test_privacy_mode_switch() {
        // 1. Start in Full mode (5-hop)
        // 2. Send a request, verify response
        // 3. Switch to Fast mode (2-hop)
        // 4. Send a request, verify response
        // 5. Compare latencies (Fast should be lower)

        let modes = [PrivacyMode::Full, PrivacyMode::Fast, PrivacyMode::None];
        for mode in &modes {
            eprintln!("Testing mode: {:?}", mode);
        }

        eprintln!("TODO: Implement with actual proxy mode switching");
    }
}

// Ensure the test file compiles even without the nym-live feature
#[cfg(not(feature = "nym-live"))]
#[test]
fn nym_integration_tests_require_nym_live_feature() {
    // This test always passes — it just confirms the file compiles.
    // Run with --features nym-live to enable actual Nym tests.
}
