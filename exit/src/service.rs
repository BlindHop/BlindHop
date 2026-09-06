//! Nym Service Provider — listens for mixnet messages and processes them.

use anyhow::Result;

use nym_sdk::mixnet::MixnetMessageSender;

use blindhop_common::error::BlindHopError;
use blindhop_common::rpc::{MessageType, MixnetMessage};

use crate::backend::{ExitBackend, SubstrateWsBackend};

/// Run the exit service as a Nym Service Provider.
///
/// Connects to the Nym mixnet, announces our address, then loops
/// receiving messages, forwarding JSON-RPC requests to the Substrate
/// full node, and sending responses back through the mixnet.
pub async fn run_exit_service(target_rpc: &str) -> Result<()> {
    tracing::info!("Connecting to Nym mixnet as Service Provider...");

    let mut client = nym_sdk::mixnet::MixnetClient::connect_new()
        .await
        .map_err(|e| BlindHopError::NymTransport(format!("Failed to connect: {}", e)))?;

    let our_address = client.nym_address().to_string();
    tracing::info!("Exit service registered on Nym network");
    tracing::info!("  Our Nym address: {}", our_address);

    // Write our address to a file for the proxy to read
    if let Err(e) = tokio::fs::write(".exit_nym_address", &our_address).await {
        tracing::warn!("Could not write .exit_nym_address file: {}", e);
    }

    // Initialize the Substrate RPC backend
    let backend = SubstrateWsBackend::new(target_rpc.to_string());
    tracing::info!("Exit service ready — forwarding to {}", target_rpc);

    // Main message processing loop
    loop {
        // wait_for_messages returns Option<Vec<ReconstructedMessage>>
        let messages = match client.wait_for_messages().await {
            Some(msgs) => msgs,
            None => {
                tracing::error!("Nym mixnet channel closed");
                anyhow::bail!("Nym mixnet channel closed unexpectedly");
            }
        };

        for received in messages {
            let sender = received.sender_tag;

            match MixnetMessage::from_bytes(&received.message) {
                Ok(msg) => {
                    if msg.msg_type != MessageType::Request {
                        tracing::debug!("Skipping non-request message");
                        continue;
                    }

                    tracing::debug!(
                        "Received request ({} bytes) from mixnet",
                        msg.payload.len()
                    );

                    // Forward to Substrate full node
                    match backend.forward_rpc(&msg.payload).await {
                        Ok(response) => {
                            // Wrap response in MixnetMessage
                            let reply = MixnetMessage::response(response);
                            let reply_bytes = reply.to_bytes();

                            // Send back through the mixnet using sender tag (SURB reply)
                            if let Some(tag) = sender {
                                if let Err(e) = client
                                    .send_reply(tag, reply_bytes)
                                    .await
                                {
                                    tracing::warn!("Failed to send reply through mixnet: {}", e);
                                }
                            } else {
                                tracing::warn!("No sender tag — cannot send reply");
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Substrate RPC forward failed: {}", e);
                            // Send error response back
                            let error_json = serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": null,
                                "error": {
                                    "code": -32000,
                                    "message": format!("Exit service error: {}", e)
                                }
                            });
                            let reply = MixnetMessage::response(
                                serde_json::to_vec(&error_json).unwrap(),
                            );
                            if let Some(tag) = sender {
                                let _ = client
                                    .send_reply(tag, reply.to_bytes())
                                    .await;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to parse MixnetMessage: {}", e);
                }
            }
        }
    }
}
