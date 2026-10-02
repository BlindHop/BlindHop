//! Nym Service Provider — listens for mixnet messages and processes them.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use nym_sdk::mixnet::{
    AnonymousSenderTag, IncludedSurbs, MixnetClient, MixnetClientBuilder, MixnetClientSender,
    MixnetMessageSender, StoragePaths,
};

use blindhop_common::rpc::{MessageType, MixnetMessage};

use crate::backend::{ExitBackend, SubstrateWsBackend};
use crate::limiter::{
    Limiter, MAX_IN_FLIGHT, MAX_IN_FLIGHT_PER_CLIENT, MAX_WAIT_FOR_SLOT, MAX_WAITING,
};
use crate::policy;

/// Run the exit service as a Nym Service Provider.
///
/// Connects to the Nym mixnet, announces our address, then loops
/// receiving messages, forwarding JSON-RPC requests to the Substrate
/// full node, and sending responses back through the mixnet.
pub async fn run_exit_service(
    target_rpc: &str,
    data_dir: &Path,
    gateway: Option<String>,
) -> Result<()> {
    tracing::info!("Connecting to Nym mixnet as Service Provider...");

    let mut client = connect_persistent(data_dir, gateway).await?;

    let our_address = client.nym_address().to_string();
    tracing::info!("Exit service registered on Nym network");
    tracing::info!("  Our Nym address: {}", our_address);

    // Write our address to a file for the proxy to read
    if let Err(e) = tokio::fs::write(".exit_nym_address", &our_address).await {
        tracing::warn!("Could not write .exit_nym_address file: {}", e);
    }

    // Initialize the Substrate RPC backend
    let backend = Arc::new(SubstrateWsBackend::new(target_rpc.to_string()));
    let sender = client.split_sender();
    let limiter = Limiter::new(MAX_IN_FLIGHT, MAX_IN_FLIGHT_PER_CLIENT, MAX_WAITING);
    tracing::info!(
        "Exit service ready — forwarding to {} via {} backend",
        target_rpc,
        backend.backend_type()
    );

    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);

    // Fires if the SDK shuts the client down after an internal failure. The
    // message channel isn't guaranteed to close then, so without this the
    // process could stay up but dead, and systemd would never restart it.
    let client_failed = client.cancellation_token();

    // The SDK can also lose its gateway connection without shutting down
    // (seen after a gateway "Internal gateway storage error"): the process
    // stays up but nothing arrives or leaves. To catch that, the exit sends
    // itself a probe through the mixnet every PROBE_INTERVAL and exits if
    // none has come back for PROBE_TIMEOUT, so systemd restarts it.
    let self_address = *client.nym_address();
    let mut probe_timer = tokio::time::interval(PROBE_INTERVAL);
    probe_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut last_probe = Instant::now();

    // Main message processing loop
    loop {
        let messages = tokio::select! {
            _ = probe_timer.tick() => {
                if last_probe.elapsed() > PROBE_TIMEOUT {
                    tracing::error!(
                        "No liveness probe came back through the mixnet in {} s; exiting",
                        last_probe.elapsed().as_secs()
                    );
                    anyhow::bail!("Nym connection stopped delivering messages");
                }
                // On its own task: if the connection is stuck, sending can
                // block, and the loop must keep running to notice.
                let sender = sender.clone();
                tokio::spawn(async move {
                    if let Err(e) = sender
                        .send_message(self_address, PROBE, IncludedSurbs::ExposeSelfAddress)
                        .await
                    {
                        tracing::warn!("Failed to send liveness probe: {}", e);
                    }
                });
                continue;
            }
            _ = &mut shutdown => {
                // A clean disconnect lets the SDK flush its reply-SURB store;
                // otherwise it's discarded as corrupted on the next start.
                tracing::info!("Shutdown signal received; disconnecting from Nym mixnet");
                client.disconnect().await;
                return Ok(());
            }
            _ = client_failed.cancelled() => {
                tracing::error!("Nym client shut down unexpectedly; exiting");
                anyhow::bail!("Nym client shut down unexpectedly");
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
            if received.sender_tag.is_none() && received.message == PROBE {
                tracing::debug!("Liveness probe came back");
                last_probe = Instant::now();
                continue;
            }
            // Replies travel on the sender's reply SURBs; without a tag there
            // is no way to answer, so don't spend any work on the message.
            let Some(tag) = received.sender_tag else {
                tracing::debug!("Dropping message without reply SURBs");
                continue;
            };
            let msg = match MixnetMessage::from_bytes(&received.message) {
                Ok(msg) if msg.msg_type == MessageType::Request => msg,
                Ok(_) => {
                    tracing::debug!("Skipping non-request message");
                    continue;
                }
                Err(e) => {
                    tracing::warn!("Failed to parse MixnetMessage: {}", e);
                    continue;
                }
            };
            tracing::debug!(
                "Received request {} ({} bytes) from mixnet",
                msg.correlation_id,
                msg.payload.len()
            );

            // Each request runs on its own task, so a slow one (e.g. a large
            // state_getMetadata) or one waiting for a slot doesn't hold up
            // everyone else.
            let (sender, backend, limiter) = (sender.clone(), backend.clone(), limiter.clone());
            tokio::spawn(async move {
                let received = std::time::Instant::now();
                let compress = msg.accepts_compression;
                // Policy first, so refused requests never wait or take a slot.
                let request = match policy::validate(&msg.payload) {
                    Ok(request) => request,
                    Err(reply) => {
                        tracing::debug!("Refused a request that failed policy checks");
                        send_reply(sender, tag, msg.correlation_id, reply, compress).await;
                        return;
                    }
                };
                let Some(_permit) = limiter.acquire(tag, MAX_WAIT_FOR_SLOT).await else {
                    tracing::warn!("Exit busy; refusing a request");
                    let reply = policy::error_reply(
                        &request.id,
                        policy::code::BUSY,
                        "Exit is busy; retry shortly",
                    );
                    send_reply(sender, tag, msg.correlation_id, reply, compress).await;
                    return;
                };
                let slot_wait = received.elapsed();
                let reply = forward(backend.as_ref(), &msg.payload, &request).await;
                let upstream = received.elapsed() - slot_wait;
                let reply_len = reply.len();
                send_reply(sender, tag, msg.correlation_id, reply, compress).await;
                tracing::debug!(
                    "Request {} answered: slot wait {} ms, upstream {} ms, total {} ms, {} bytes",
                    msg.correlation_id,
                    slot_wait.as_millis(),
                    upstream.as_millis(),
                    received.elapsed().as_millis(),
                    reply_len
                );
            });
        }
    }
}

/// Forward a validated request and build the reply payload. Errors carry
/// the request's `id`.
async fn forward<B: ExitBackend + ?Sized>(
    backend: &B,
    payload: &[u8],
    request: &policy::ValidRequest,
) -> Vec<u8> {
    match backend.forward_rpc(payload, &request.id).await {
        Ok(response) => response,
        Err(e) => {
            tracing::warn!("Upstream request failed: {}", e);
            policy::error_reply(&request.id, policy::code::UPSTREAM_ERROR, &e.to_string())
        }
    }
}

/// Send `payload` back to the client behind `tag` via its reply SURBs,
/// compressed if the client said it can read compressed replies (fewer
/// packets, so fewer reply SURBs and a faster reply).
async fn send_reply(
    sender: MixnetClientSender,
    tag: AnonymousSenderTag,
    correlation_id: u64,
    payload: Vec<u8>,
    compress: bool,
) {
    let reply = MixnetMessage::response(correlation_id, payload);
    let bytes = if compress {
        reply.to_bytes_compressed()
    } else {
        reply.to_bytes()
    };
    if let Err(e) = sender.send_reply(tag, bytes).await {
        tracing::warn!("Failed to send reply through mixnet: {}", e);
    }
}

/// Connect with keys stored in `data_dir`, so the exit's Nym address stays
/// the same across restarts. Keys are generated on first run.
async fn connect_persistent(data_dir: &Path, gateway: Option<String>) -> Result<MixnetClient> {
    create_private_dir(data_dir)?;

    let storage_paths = StoragePaths::new_from_dir(data_dir)
        .with_context(|| format!("Invalid data dir {}", data_dir.display()))?;

    let mut builder = MixnetClientBuilder::new_with_default_storage(storage_paths)
        .await
        .context("Failed to open Nym client storage")?
        .debug_config(debug_config());
    if let Some(gateway) = gateway {
        tracing::info!("  Requested gateway: {}", gateway);
        builder = builder.request_gateway(gateway);
    }
    builder
        .build()
        .context("Failed to build Nym client")?
        .connect_to_mixnet()
        .await
        .context("Failed to connect to Nym mixnet")
}

/// Nym client settings for the exit.
///
/// - Every reply packet uses one reply SURB supplied by the proxy. A ~1 MB
///   response is several hundred packets; with the SDK default of at most 50
///   SURBs per top-up request, large replies took ~60 s and hit the proxy's
///   timeout. 500 is the most a client allows by default
///   (`maximum_allowed_reply_surb_request_size`).
/// - Replies are sent as soon as they're ready instead of on the SDK's
///   Poisson schedule. With Poisson sending, every idle slot is filled with a
///   cover packet: at the default 20 ms that's a constant ~50 packets/s, and
///   raising the rate for faster replies (5 ms) made the idle exit send
///   ~617 KiB/s (~55 GB/day) to its gateway, which led to periodic 45-60 s
///   stalls. The exit is a public service; its users' anonymity comes from
///   their own clients and the mixnet (replies still travel on their SURBs),
///   so it doesn't need a constant-rate stream. The low-rate loop cover
///   traffic stays on.
fn debug_config() -> nym_sdk::DebugConfig {
    let mut config = nym_sdk::DebugConfig::default();
    config.reply_surbs.maximum_reply_surb_request_size = MAX_REPLY_SURB_REQUEST;
    config.traffic.disable_main_poisson_packet_distribution = true;
    config
}

const MAX_REPLY_SURB_REQUEST: u32 = 500;

/// Message the exit sends itself to check its mixnet connection still works.
/// Anyone could send the same bytes, but only while messages are getting
/// through, which is all the probe checks.
const PROBE: &[u8] = b"blindhop-exit-liveness-probe";
/// How often to send a probe. A probe is one small packet each way.
const PROBE_INTERVAL: Duration = Duration::from_secs(60);
/// How long without a probe arriving before the exit gives up. Allows for a
/// couple of lost probes and the slow trips seen during gateway hiccups.
const PROBE_TIMEOUT: Duration = Duration::from_secs(180);

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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use blindhop_common::error::{BlindHopError, Result as BhResult};
    use serde_json::Value;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Backend that counts calls and answers or fails on demand.
    struct FakeBackend {
        calls: AtomicUsize,
        fail: bool,
    }

    #[async_trait]
    impl ExitBackend for FakeBackend {
        async fn forward_rpc(&self, _request: &[u8], id: &Value) -> BhResult<Vec<u8>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                return Err(BlindHopError::SubstrateRpc("node down".into()));
            }
            Ok(serde_json::to_vec(
                &serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": "ok" }),
            )
            .unwrap())
        }
        fn backend_type(&self) -> &str {
            "fake"
        }
    }

    fn backend(fail: bool) -> FakeBackend {
        FakeBackend {
            calls: AtomicUsize::new(0),
            fail,
        }
    }

    /// Same order as the exit's request task: policy, then forward.
    async fn run(backend: &FakeBackend, payload: &str) -> Value {
        let reply = match policy::validate(payload.as_bytes()) {
            Ok(request) => forward(backend, payload.as_bytes(), &request).await,
            Err(reply) => reply,
        };
        serde_json::from_slice(&reply).unwrap()
    }

    #[tokio::test]
    async fn allowed_request_is_forwarded() {
        let b = backend(false);
        let r = run(&b, r#"{"jsonrpc":"2.0","id":4,"method":"system_chain"}"#).await;
        assert_eq!(r["result"], "ok");
        assert_eq!(b.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn refused_requests_never_reach_the_node() {
        let b = backend(false);
        for payload in [
            r#"{"jsonrpc":"2.0","id":1,"method":"author_insertKey","params":[]}"#,
            r#"[{"jsonrpc":"2.0","id":1,"method":"system_chain"}]"#,
            r#"{"jsonrpc":"2.0","method":"system_chain"}"#,
        ] {
            assert!(run(&b, payload).await.get("error").is_some(), "{payload}");
        }
        assert_eq!(b.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn upstream_failure_is_an_error_with_the_request_id() {
        let b = backend(true);
        let r = run(&b, r#"{"jsonrpc":"2.0","id":"q1","method":"system_chain"}"#).await;
        assert_eq!(r["id"], "q1");
        assert_eq!(r["error"]["code"], policy::code::UPSTREAM_ERROR);
    }
}
