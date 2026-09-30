//! Exit backend trait and implementations.
//!
//! Provides a modular interface for the exit service to forward
//! JSON-RPC requests to different backends:
//! - `SubstrateWsBackend`: Direct WebSocket to a Substrate full node (default)
//! - Future: `Socks5Backend` for routing through Nym's exit infrastructure

use async_trait::async_trait;
use serde_json::Value;

use blindhop_common::error::{BlindHopError, Result};

use crate::substrate_rpc::UpstreamPool;

/// Trait for exit service backends.
///
/// Implementors handle the actual forwarding of JSON-RPC requests
/// to the target Substrate full node.
#[async_trait]
pub trait ExitBackend: Send + Sync {
    /// Forward a JSON-RPC request whose `id` is `id` and return the response.
    async fn forward_rpc(&self, request: &[u8], id: &Value) -> Result<Vec<u8>>;

    /// Get the backend type name.
    fn backend_type(&self) -> &str;
}

/// WebSocket backend connecting directly to a Substrate full node.
pub struct SubstrateWsBackend {
    pool: UpstreamPool,
}

impl SubstrateWsBackend {
    /// Create a new backend targeting the given Substrate full node URL.
    pub fn new(target_url: String) -> Self {
        Self {
            pool: UpstreamPool::new(target_url),
        }
    }
}

#[async_trait]
impl ExitBackend for SubstrateWsBackend {
    async fn forward_rpc(&self, request: &[u8], id: &Value) -> Result<Vec<u8>> {
        self.pool
            .request(request, id)
            .await
            .map_err(|e| BlindHopError::SubstrateRpc(e.to_string()))
    }

    fn backend_type(&self) -> &str {
        "substrate-ws"
    }
}
