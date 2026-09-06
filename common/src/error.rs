//! Error types for BlindHop.

use thiserror::Error;

/// Top-level error type for BlindHop operations.
#[derive(Debug, Error)]
pub enum BlindHopError {
    /// Error establishing or using the Nym mixnet connection.
    #[error("Nym transport error: {0}")]
    NymTransport(String),

    /// Error connecting to or communicating with the Substrate full node.
    #[error("Substrate RPC error: {0}")]
    SubstrateRpc(String),

    /// Error in the WebSocket proxy layer.
    #[error("Proxy error: {0}")]
    Proxy(String),

    /// Error in the exit service.
    #[error("Exit service error: {0}")]
    ExitService(String),

    /// JSON-RPC serialization/deserialization error.
    #[error("JSON-RPC error: {0}")]
    JsonRpc(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Privacy mode cannot be satisfied (e.g., Nym network unavailable).
    #[error("Privacy requirement not met: {0}")]
    PrivacyUnavailable(String),

    /// WebSocket error.
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Convenience Result type for BlindHop operations.
pub type Result<T> = std::result::Result<T, BlindHopError>;
