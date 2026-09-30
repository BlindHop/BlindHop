//! Nym Service Provider — listens for mixnet messages and processes them.

use std::path::Path;

use anyhow::{Context, Result};

use nym_sdk::mixnet::{MixnetClient, MixnetClientBuilder, MixnetMessageSender, StoragePaths};

use blindhop_common::rpc::{MessageType, MixnetMessage};

use crate::backend::{ExitBackend, SubstrateWsBackend};

/// Run the exit service as a Nym Service Provider.
///
/// Connects to the Nym mixnet, announces our address, then loops
/// receiving messages, forwarding JSON-RPC requests to the Substrate
/// full node, and sending responses back through the mixnet.
pub async fn run_exit_service(target_rpc: &str, data_dir: &Path) -> Result<()> {
    tracing::info!("Connecting to Nym mixnet as Service Provider...");

    let mut client = connect_persistent(data_dir).await?;

    let our_address = client.nym_address().to_string();
    tracing::info!("Exit service registered on Nym network");
    tracing::info!("  Our Nym address: {}", our_address);

    // Write our address to a file for the proxy to read
    if let Err(e) = tokio::fs::write(".exit_nym_address", &our_address).await {
        tracing::warn!("Could not write .exit_nym_address file: {}", e);
    }

    // Initialize the Substrate RPC backend
    let backend = SubstrateWsBackend::new(target_rpc.to_string());
    tracing::info!(
        "Exit service ready — forwarding to {} via {} backend",
        target_rpc,
        backend.backend_type()
    );

    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);

    // Main message processing loop
    loop {
        let messages = tokio::select! {
            _ = &mut shutdown => {
                // A clean disconnect lets the SDK flush its reply-SURB store;
                // otherwise it's discarded as corrupted on the next start.
                tracing::info!("Shutdown signal received; disconnecting from Nym mixnet");
                client.disconnect().await;
                return Ok(());
            }
            // wait_for_messages returns Option<Vec<ReconstructedMessage>>
            batch = client.wait_for_messages() => match batch {
                Some(msgs) => msgs,
                None => {
                    tracing::error!("Nym mixnet channel closed");
                    anyhow::bail!("Nym mixnet channel closed unexpectedly");
                }
            },
        };

        for received in messages {
            let sender = received.sender_tag;

            match MixnetMessage::from_bytes(&received.message) {
                Ok(msg) => {
                    if msg.msg_type != MessageType::Request {
                        tracing::debug!("Skipping non-request message");
                        continue;
                    }

                    tracing::debug!("Received request ({} bytes) from mixnet", msg.payload.len());

                    // Forward to Substrate full node
                    match backend.forward_rpc(&msg.payload).await {
                        Ok(response) => {
                            // Wrap response in MixnetMessage
                            let reply = MixnetMessage::response(msg.correlation_id, response);
                            let reply_bytes = reply.to_bytes();

                            // Send back through the mixnet using sender tag (SURB reply)
                            if let Some(tag) = sender {
                                if let Err(e) = client.send_reply(tag, reply_bytes).await {
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
                                msg.correlation_id,
                                serde_json::to_vec(&error_json).unwrap(),
                            );
                            if let Some(tag) = sender {
                                let _ = client.send_reply(tag, reply.to_bytes()).await;
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

/// Connect with keys stored in `data_dir`, so the exit's Nym address stays
/// the same across restarts. Keys are generated on first run.
async fn connect_persistent(data_dir: &Path) -> Result<MixnetClient> {
    create_private_dir(data_dir)?;

    let storage_paths = StoragePaths::new_from_dir(data_dir)
        .with_context(|| format!("Invalid data dir {}", data_dir.display()))?;

    MixnetClientBuilder::new_with_default_storage(storage_paths)
        .await
        .context("Failed to open Nym client storage")?
        .build()
        .context("Failed to build Nym client")?
        .connect_to_mixnet()
        .await
        .context("Failed to connect to Nym mixnet")
}

/// Create `dir` if needed, readable only by the owner (it holds private keys).
fn create_private_dir(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir)
        .with_context(|| format!("Failed to create data dir {}", dir.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
            .with_context(|| format!("Failed to set permissions on {}", dir.display()))?;
    }

    Ok(())
}

/// Resolves on Ctrl-C, or SIGTERM on Unix (what systemd and Docker send).
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Ok(mut sigterm) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = sigterm.recv() => {}
                }
                return;
            }
            Err(e) => tracing::warn!("Could not listen for SIGTERM: {}", e),
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}
