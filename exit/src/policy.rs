//! What the exit is willing to forward to the full node.
//!
//! The exit is reachable anonymously by anyone who knows its Nym address, so
//! it only forwards single JSON-RPC requests for an allowlisted set of
//! read-only methods (plus extrinsic submission), within size limits.

use serde_json::Value;

/// Largest request payload the exit accepts.
pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;

/// Largest response the exit reads from the full node. Every reply packet
/// costs a reply SURB and memory while in flight, so this bounds both.
/// (`state_getMetadata` is ~1.2 MB.)
pub const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

/// JSON-RPC methods the exit forwards.
///
/// Deliberately excluded: node-administration and node-identifying methods
/// (e.g. `author_insertKey`, `author_rotateKeys`, `system_peers`,
/// `system_localPeerId`), unbounded queries (`state_getKeys`,
/// `state_getPairs`), and all subscriptions, which a request/response exit
/// can't carry.
pub const ALLOWED_METHODS: &[&str] = &[
    // Node and chain information
    "rpc_methods",
    "system_chain",
    "system_chainType",
    "system_health",
    "system_name",
    "system_properties",
    "system_version",
    // Blocks
    "chain_getBlock",
    "chain_getBlockHash",
    "chain_getFinalizedHead",
    "chain_getHeader",
    // State (read-only)
    "state_call",
    "state_getKeysPaged",
    "state_getMetadata",
    "state_getReadProof",
    "state_getRuntimeVersion",
    "state_getStorage",
    "state_getStorageHash",
    "state_getStorageSize",
    "state_queryStorageAt",
    // Accounts, fees and transactions
    "author_submitExtrinsic",
    "payment_queryFeeDetails",
    "payment_queryInfo",
    "system_accountNextIndex",
    // New JSON-RPC spec (non-subscription)
    "chainSpec_v1_chainName",
    "chainSpec_v1_genesisHash",
    "chainSpec_v1_properties",
];

/// JSON-RPC error codes used by the exit.
pub mod code {
    pub const INVALID_REQUEST: i64 = -32600;
    pub const METHOD_NOT_ALLOWED: i64 = -32601;
    /// Upstream full-node failure (connect, timeout, oversized response).
    pub const UPSTREAM_ERROR: i64 = -32000;
    /// Too many requests in flight; retry later.
    pub const BUSY: i64 = -32005;
}

/// A request that passed validation.
#[derive(Debug, PartialEq)]
pub struct ValidRequest {
    /// The request's JSON-RPC `id`, echoed in any error reply.
    pub id: Value,
    pub method: String,
}

/// Check a request payload against the exit's policy.
///
/// On rejection, returns the complete JSON-RPC error reply to send back.
pub fn validate(payload: &[u8]) -> Result<ValidRequest, Vec<u8>> {
    if payload.len() > MAX_REQUEST_BYTES {
        return Err(error_reply(
            &Value::Null,
            code::INVALID_REQUEST,
            &format!("Request too large (limit {} bytes)", MAX_REQUEST_BYTES),
        ));
    }

    let request: Value = serde_json::from_slice(payload).map_err(|_| {
        error_reply(
            &Value::Null,
            code::INVALID_REQUEST,
            "Request is not valid JSON",
        )
    })?;
    let Value::Object(request) = request else {
        return Err(error_reply(
            &Value::Null,
            code::INVALID_REQUEST,
            "Batch requests are not supported; send one request object",
        ));
    };

    // Notifications (no id) get no reply from the node, so the exit would
    // hold resources until its timeout.
    let id = match request.get("id") {
        Some(id @ (Value::Number(_) | Value::String(_))) => id.clone(),
        _ => {
            return Err(error_reply(
                &Value::Null,
                code::INVALID_REQUEST,
                "Request must have a numeric or string id",
            ));
        }
    };

    let Some(method) = request.get("method").and_then(Value::as_str) else {
        return Err(error_reply(
            &id,
            code::INVALID_REQUEST,
            "Request has no method",
        ));
    };
    if !ALLOWED_METHODS.contains(&method) {
        return Err(error_reply(
            &id,
            code::METHOD_NOT_ALLOWED,
            &format!("Method not allowed by this exit: {}", method),
        ));
    }

    Ok(ValidRequest {
        id,
        method: method.to_string(),
    })
}

/// Best-effort `id` of a request, for replies to requests that were not
/// validated (e.g. when the exit is busy).
pub fn request_id(payload: &[u8]) -> Value {
    if payload.len() > MAX_REQUEST_BYTES {
        return Value::Null;
    }
    serde_json::from_slice::<Value>(payload)
        .ok()
        .and_then(|v| v.get("id").cloned())
        .unwrap_or(Value::Null)
}

/// Build a JSON-RPC error reply.
pub fn error_reply(id: &Value, code: i64, message: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    }))
    .expect("JSON-RPC error serialization should not fail")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn reply(bytes: Vec<u8>) -> Value {
        serde_json::from_slice(&bytes).unwrap()
    }

    fn rejected(payload: &[u8]) -> Value {
        reply(validate(payload).expect_err("should be rejected"))
    }

    #[test]
    fn allowed_method_passes() {
        let req = br#"{"jsonrpc":"2.0","id":7,"method":"chain_getHeader","params":[]}"#;
        assert_eq!(
            validate(req),
            Ok(ValidRequest {
                id: json!(7),
                method: "chain_getHeader".into()
            })
        );
        let req =
            br#"{"jsonrpc":"2.0","id":"abc","method":"author_submitExtrinsic","params":["0x00"]}"#;
        assert_eq!(validate(req).unwrap().id, json!("abc"));
    }

    #[test]
    fn unsafe_and_subscription_methods_are_refused_with_id() {
        for method in [
            "author_insertKey",
            "author_rotateKeys",
            "system_peers",
            "state_getKeys",
            "chain_subscribeNewHeads",
            "chainHead_v1_follow",
        ] {
            let req = format!(r#"{{"jsonrpc":"2.0","id":3,"method":"{}"}}"#, method);
            let r = rejected(req.as_bytes());
            assert_eq!(r["error"]["code"], code::METHOD_NOT_ALLOWED, "{method}");
            assert_eq!(r["id"], 3, "error must echo the request id");
        }
    }

    #[test]
    fn malformed_requests_are_refused() {
        let batch = br#"[{"jsonrpc":"2.0","id":1,"method":"system_chain"}]"#;
        assert_eq!(rejected(batch)["error"]["code"], code::INVALID_REQUEST);

        let notification = br#"{"jsonrpc":"2.0","method":"system_chain"}"#;
        assert_eq!(
            rejected(notification)["error"]["code"],
            code::INVALID_REQUEST
        );

        let null_id = br#"{"jsonrpc":"2.0","id":null,"method":"system_chain"}"#;
        assert_eq!(rejected(null_id)["error"]["code"], code::INVALID_REQUEST);

        let no_method = br#"{"jsonrpc":"2.0","id":5}"#;
        let r = rejected(no_method);
        assert_eq!(r["error"]["code"], code::INVALID_REQUEST);
        assert_eq!(r["id"], 5);

        assert_eq!(
            rejected(b"not json")["error"]["code"],
            code::INVALID_REQUEST
        );
    }

    #[test]
    fn oversized_request_is_refused_without_parsing() {
        let big = vec![b' '; MAX_REQUEST_BYTES + 1];
        let r = rejected(&big);
        assert_eq!(r["error"]["code"], code::INVALID_REQUEST);
        assert_eq!(r["id"], Value::Null);
    }

    #[test]
    fn request_id_is_best_effort() {
        assert_eq!(request_id(br#"{"id":9,"method":"x"}"#), json!(9));
        assert_eq!(request_id(b"garbage"), Value::Null);
    }
}
