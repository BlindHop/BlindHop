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

    /// Chosen by the sender of a request and echoed unchanged in the
    /// response, so replies can be matched to requests. Many requests share
    /// one mixnet client, and replies can arrive in any order.
    #[serde(default)]
    pub correlation_id: u64,

    /// Set on requests whose sender can read compressed responses. The exit
    /// compresses a response only when its request set this.
    #[serde(default)]
    pub accepts_compression: bool,
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
    pub fn request(correlation_id: u64, payload: Vec<u8>) -> Self {
        Self {
            payload,
            msg_type: MessageType::Request,
            correlation_id,
            accepts_compression: false,
        }
    }

    /// Mark a request as able to read compressed responses.
    pub fn accepting_compression(mut self) -> Self {
        self.accepts_compression = true;
        self
    }

    /// Create a response message, echoing the request's correlation ID.
    pub fn response(correlation_id: u64, payload: Vec<u8>) -> Self {
        Self {
            payload,
            msg_type: MessageType::Response,
            correlation_id,
            accepts_compression: false,
        }
    }

    /// Serialize to bytes for transmission through the mixnet.
    ///
    /// Wire format: `[type: u8][correlation_id: u64 big-endian][payload]`.
    /// The low nibble of `type` is `0x01` = request or `0x02` = response;
    /// bit `0x40` (requests) = sender accepts compressed responses; bit
    /// `0x80` = payload is deflate-raw compressed. Every mixnet packet costs
    /// a reply SURB, so payloads are sent as raw bytes; the older JSON
    /// envelope encoded each byte as a decimal number, about 3.5× larger.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.frame(self.type_byte(), &self.payload)
    }

    /// Like [`to_bytes`](Self::to_bytes), but deflate-compresses the payload
    /// when that makes it smaller. Use only for replies to requests with
    /// `accepts_compression`.
    pub fn to_bytes_compressed(&self) -> Vec<u8> {
        if self.payload.len() >= MIN_COMPRESS_BYTES
            && let Some(compressed) = deflate(&self.payload)
            && compressed.len() < self.payload.len()
        {
            return self.frame(self.type_byte() | FLAG_COMPRESSED, &compressed);
        }
        self.to_bytes()
    }

    fn type_byte(&self) -> u8 {
        match self.msg_type {
            MessageType::Request if self.accepts_compression => {
                FRAME_TYPE_REQUEST | FLAG_ACCEPTS_COMPRESSION
            }
            MessageType::Request => FRAME_TYPE_REQUEST,
            MessageType::Response => FRAME_TYPE_RESPONSE,
        }
    }

    fn frame(&self, type_byte: u8, payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(FRAME_HEADER_LEN + payload.len());
        out.push(type_byte);
        out.extend_from_slice(&self.correlation_id.to_be_bytes());
        out.extend_from_slice(payload);
        out
    }

    /// Deserialize from bytes received from the mixnet, decompressing the
    /// payload if needed (up to [`MAX_DECOMPRESSED_BYTES`]).
    ///
    /// Also accepts the older JSON envelope (`{"payload":[..],"msg_type":..}`),
    /// which starts with `{` and so can't be mistaken for a binary frame.
    pub fn from_bytes(data: &[u8]) -> crate::error::Result<Self> {
        let invalid = |msg: String| crate::error::BlindHopError::JsonRpc(msg);

        if data.first() == Some(&b'{') {
            return serde_json::from_slice(data).map_err(|e| invalid(e.to_string()));
        }
        if data.len() < FRAME_HEADER_LEN {
            return Err(invalid(format!(
                "Mixnet frame too short: {} bytes",
                data.len()
            )));
        }

        let type_byte = data[0];
        let unknown = type_byte & !(FRAME_TYPE_MASK | FLAG_ACCEPTS_COMPRESSION | FLAG_COMPRESSED);
        let msg_type = match type_byte & FRAME_TYPE_MASK {
            FRAME_TYPE_REQUEST if unknown == 0 => MessageType::Request,
            FRAME_TYPE_RESPONSE if unknown == 0 => MessageType::Response,
            _ => {
                return Err(invalid(format!(
                    "Unknown mixnet frame type {type_byte:#04x}"
                )));
            }
        };
        let mut id = [0u8; 8];
        id.copy_from_slice(&data[1..FRAME_HEADER_LEN]);

        let body = &data[FRAME_HEADER_LEN..];
        let payload = if type_byte & FLAG_COMPRESSED != 0 {
            inflate(body, MAX_DECOMPRESSED_BYTES).map_err(invalid)?
        } else {
            body.to_vec()
        };

        Ok(Self {
            payload,
            msg_type,
            correlation_id: u64::from_be_bytes(id),
            accepts_compression: msg_type == MessageType::Request
                && type_byte & FLAG_ACCEPTS_COMPRESSION != 0,
        })
    }
}

const FRAME_TYPE_REQUEST: u8 = 0x01;
const FRAME_TYPE_RESPONSE: u8 = 0x02;
const FRAME_TYPE_MASK: u8 = 0x0F;
/// Request flag: the sender accepts compressed responses.
const FLAG_ACCEPTS_COMPRESSION: u8 = 0x40;
/// The payload is deflate-raw compressed.
const FLAG_COMPRESSED: u8 = 0x80;
/// Type byte + 8-byte correlation ID.
const FRAME_HEADER_LEN: usize = 9;
/// Payloads smaller than this aren't worth compressing.
const MIN_COMPRESS_BYTES: usize = 1024;

/// Largest payload a compressed frame may expand to. Guards against
/// decompression bombs (a tiny frame that inflates to gigabytes).
pub const MAX_DECOMPRESSED_BYTES: usize = 8 * 1024 * 1024;

fn deflate(data: &[u8]) -> Option<Vec<u8>> {
    use std::io::Write;
    let mut encoder =
        flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(data).ok()?;
    encoder.finish().ok()
}

fn inflate(data: &[u8], limit: usize) -> std::result::Result<Vec<u8>, String> {
    use std::io::Read;
    let mut out = Vec::new();
    flate2::read::DeflateDecoder::new(data)
        .take(limit as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|e| format!("Invalid compressed payload: {e}"))?;
    if out.len() > limit {
        return Err(format!("Decompressed payload exceeds {limit} bytes"));
    }
    Ok(out)
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
        let msg = MixnetMessage::request(42, b"hello".to_vec());
        let bytes = msg.to_bytes();
        let parsed = MixnetMessage::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.payload, b"hello");
        assert_eq!(parsed.msg_type, MessageType::Request);
        assert_eq!(parsed.correlation_id, 42);
    }

    #[test]
    fn test_mixnet_frame_layout() {
        let bytes = MixnetMessage::response(0x0102030405060708, b"{}".to_vec()).to_bytes();
        assert_eq!(bytes, [0x02, 1, 2, 3, 4, 5, 6, 7, 8, b'{', b'}']);

        let parsed = MixnetMessage::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.msg_type, MessageType::Response);
        assert_eq!(parsed.correlation_id, 0x0102030405060708);
        assert_eq!(parsed.payload, b"{}");
    }

    #[test]
    fn test_mixnet_frame_empty_payload() {
        let bytes = MixnetMessage::request(7, vec![]).to_bytes();
        assert_eq!(bytes.len(), FRAME_HEADER_LEN);
        assert!(
            MixnetMessage::from_bytes(&bytes)
                .unwrap()
                .payload
                .is_empty()
        );
    }

    #[test]
    fn test_mixnet_frame_rejects_malformed() {
        assert!(MixnetMessage::from_bytes(b"").is_err());
        assert!(MixnetMessage::from_bytes(&[0x01, 0, 0]).is_err());
        assert!(MixnetMessage::from_bytes(&[0x7F, 0, 0, 0, 0, 0, 0, 0, 1]).is_err());
        // Unknown flag bits
        assert!(MixnetMessage::from_bytes(&[0x21, 0, 0, 0, 0, 0, 0, 0, 1]).is_err());
        // Compressed flag with garbage body
        assert!(MixnetMessage::from_bytes(&[0x82, 0, 0, 0, 0, 0, 0, 0, 1, 0xff, 0xfe]).is_err());
    }

    #[test]
    fn test_request_compression_flag() {
        let bytes = MixnetMessage::request(3, b"{}".to_vec())
            .accepting_compression()
            .to_bytes();
        assert_eq!(bytes[0], 0x41);
        assert!(
            MixnetMessage::from_bytes(&bytes)
                .unwrap()
                .accepts_compression
        );

        let plain = MixnetMessage::request(3, b"{}".to_vec()).to_bytes();
        assert_eq!(plain[0], 0x01);
        assert!(
            !MixnetMessage::from_bytes(&plain)
                .unwrap()
                .accepts_compression
        );
    }

    #[test]
    fn test_large_response_is_compressed_and_roundtrips() {
        // Hex text, like state_getMetadata results, compresses well.
        let payload: Vec<u8> = (0..200_000u32)
            .flat_map(|i| format!("{:02x}", (i * 7 % 251) as u8).into_bytes())
            .collect();
        let msg = MixnetMessage::response(9, payload.clone());
        let compressed = msg.to_bytes_compressed();
        assert_eq!(compressed[0], 0x82);
        assert!(
            compressed.len() < payload.len() / 2,
            "{} vs {}",
            compressed.len(),
            payload.len()
        );

        let parsed = MixnetMessage::from_bytes(&compressed).unwrap();
        assert_eq!(parsed.payload, payload);
        assert_eq!(parsed.correlation_id, 9);
    }

    #[test]
    fn test_small_response_is_not_compressed() {
        let bytes = MixnetMessage::response(1, b"{\"result\":1}".to_vec()).to_bytes_compressed();
        assert_eq!(bytes[0], 0x02);
    }

    #[test]
    fn test_decompression_bomb_is_rejected() {
        let huge = vec![0u8; MAX_DECOMPRESSED_BYTES + 1];
        let bomb = MixnetMessage::response(1, huge).to_bytes_compressed();
        assert!(bomb.len() < 64 * 1024, "zeros compress to a tiny frame");
        assert!(MixnetMessage::from_bytes(&bomb).is_err());
    }

    #[test]
    fn test_legacy_json_envelope_still_parses() {
        let json = r#"{"payload":[104,105],"msg_type":"Response","correlation_id":9}"#;
        let parsed = MixnetMessage::from_bytes(json.as_bytes()).unwrap();
        assert_eq!(parsed.payload, b"hi");
        assert_eq!(parsed.msg_type, MessageType::Response);
        assert_eq!(parsed.correlation_id, 9);
    }

    #[test]
    fn test_mixnet_message_without_correlation_id() {
        // Senders that predate correlation IDs still parse, with ID 0.
        let json = r#"{"payload":[104,105],"msg_type":"Request"}"#;
        let parsed = MixnetMessage::from_bytes(json.as_bytes()).unwrap();
        assert_eq!(parsed.payload, b"hi");
        assert_eq!(parsed.correlation_id, 0);
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
