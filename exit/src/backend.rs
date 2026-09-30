//! Exit backend trait and implementations.
//!
//! Provides a modular interface for the exit service to forward
//! JSON-RPC requests to different backends:
//! - `SubstrateWsBackend`: Direct WebSocket to a Substrate full node (default)
//! - Future: `Socks5Backend` for routing through Nym's exit infrastructure

use async_trait::async_trait;

use blindhop_common::error::Result;

use crate::substrate_rpc;

/// Trait for exit service backends.
///
/// Implementors handle the actual forwarding of JSON-RPC requests
/// to the target Substrate full node.
#[async_trait]
pub trait ExitBackend: Send + Sync {
    /// Forward a JSON-RPC request and return the response.
    async fn forward_rpc(&self, request: &[u8]) -> Result<Vec<u8>>;

    /// Get the backend type name.
    fn backend_type(&self) -> &str;
}

/// WebSocket backend connecting directly to a Substrate full node.
pub struct SubstrateWsBackend {
    target_url: String,
}

impl SubstrateWsBackend {
    /// Create a new backend targeting the given Substrate full node URL.
    pub fn new(target_url: String) -> Self {
        Self { target_url }
    }
}

#[async_trait]
impl ExitBackend for SubstrateWsBackend {
    async fn forward_rpc(&self, request: &[u8]) -> Result<Vec<u8>> {
        substrate_rpc::forward_rpc_message(request, &self.target_url).await
    }

    fn backend_type(&self) -> &str {
        "substrate-ws"
    }
}
