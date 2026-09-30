//! JSON-RPC message types for Substrate chain communication.

use serde::{Deserialize, Serialize};

/// A JSON-RPC 2.0 request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<serde_json::Value>,
}

impl JsonRpcRequest {
    /// Create a new JSON-RPC request.
    pub fn new(id: u64, method: impl Into<String>, params: Vec<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params,
        }
    }

    /// Create a simple request with no parameters.
    pub fn simple(id: u64, method: impl Into<String>) -> Self {
        Self::new(id, method, vec![])
    }

    /// Serialize to JSON bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("JsonRpcRequest serialization should not fail")
    }
}

/// A JSON-RPC 2.0 response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// A JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcResponse {
    /// Check if this response is an error.
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Get the result value, if present.
    pub fn result(&self) -> Option<&serde_json::Value> {
        self.result.as_ref()
    }

    /// Get the request ID.
    pub fn id(&self) -> Option<u64> {
        self.id
    }
}

/// Envelope for messages sent through the mixnet.
///
/// Wraps a JSON-RPC message with metadata for routing and correlation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixnetMessage {
    /// The raw JSON-RPC payload (request or response).
    pub payload: Vec<u8>,

    /// Message type for the exit service to distinguish requests from responses.
    pub msg_type: MessageType,
}

/// Type of message flowing through the mixnet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    /// A JSON-RPC request destined for the Substrate full node.
    Request,
    /// A JSON-RPC response coming back from the full node.
    Response,
}

impl MixnetMessage {
    /// Create a request message.
    pub fn request(payload: Vec<u8>) -> Self {
        Self {
            payload,
            msg_type: MessageType::Request,
        }
    }

    /// Create a response message.
    pub fn response(payload: Vec<u8>) -> Self {
        Self {
            payload,
            msg_type: MessageType::Response,
        }
    }

    /// Serialize to bytes for transmission through the mixnet.
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("MixnetMessage serialization should not fail")
    }

    /// Deserialize from bytes received from the mixnet.
    pub fn from_bytes(data: &[u8]) -> crate::error::Result<Self> {
        serde_json::from_slice(data)
            .map_err(|e| crate::error::BlindHopError::JsonRpc(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_request_roundtrip() {
        let req = JsonRpcRequest::simple(1, "system_chain");
        let json = serde_json::to_string(&req).unwrap();
        let parsed: JsonRpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, 1);
        assert_eq!(parsed.method, "system_chain");
    }

    #[test]
    fn test_json_rpc_request_with_params() {
        let req = JsonRpcRequest::new(
            42,
            "chain_getBlockHash",
            vec![serde_json::Value::Number(100.into())],
        );
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("chain_getBlockHash"));
        assert!(json.contains("100"));
    }

    #[test]
    fn test_mixnet_message_roundtrip() {
        let msg = MixnetMessage::request(b"hello".to_vec());
        let bytes = msg.to_bytes();
        let parsed = MixnetMessage::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.payload, b"hello");
        assert_eq!(parsed.msg_type, MessageType::Request);
    }

    #[test]
    fn test_json_rpc_response_parsing() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":"Paseo Asset Hub"}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id(), Some(1));
        assert!(!resp.is_error());
        assert!(resp.result().is_some());
    }

    #[test]
    fn test_json_rpc_error_response() {
        let json =
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert!(resp.is_error());
        assert!(resp.result().is_none());
    }
}
