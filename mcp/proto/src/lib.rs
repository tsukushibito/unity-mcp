use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct RpcRequest {
    jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl RpcRequest {
    pub fn new(id: Value, method: String, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method,
            params,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum RpcResponsePayload {
    Result(Value),
    Error(RpcError),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct RpcResponse {
    jsonrpc: String,
    pub id: Value,
    #[serde(flatten)]
    pub payload: RpcResponsePayload,
}

impl RpcResponse {
    pub fn new(id: Value, payload: RpcResponsePayload) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            payload,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("frame too short")]
    Truncated,
    #[error("payload too large")]
    PayloadTooLarge,
    #[error("invalid json")]
    Malformed(#[from] serde_json::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    #[error("payload too large")]
    PayloadTooLarge,
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

const HEADER_SIZE: usize = std::mem::size_of::<u32>();
const MAX_PAYLOAD_SIZE: usize = 64 * 1024; // 64 KB

pub fn encode_frame<T: Serialize>(message: &T) -> Result<Vec<u8>, EncodeError> {
    let payload = serde_json::to_vec(message)?;
    if payload.len() > MAX_PAYLOAD_SIZE {
        return Err(EncodeError::PayloadTooLarge);
    }
    let len = payload.len() as u32;
    let mut frame = Vec::with_capacity(HEADER_SIZE + payload.len());
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

pub fn decode_frame<T: for<'de> Deserialize<'de>>(data: &[u8]) -> Result<T, FrameError> {
    if data.len() < HEADER_SIZE {
        return Err(FrameError::Truncated);
    }
    let (header, body) = data.split_at(HEADER_SIZE);
    let len_bytes: [u8; HEADER_SIZE] = header.try_into().expect("slice has guaranteed size");
    let len = u32::from_le_bytes(len_bytes) as usize;

    if len > MAX_PAYLOAD_SIZE {
        return Err(FrameError::PayloadTooLarge);
    }

    let payload = body.get(..len).ok_or(FrameError::Truncated)?;
    Ok(serde_json::from_slice(payload)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_request() {
        let req = RpcRequest::new(
            Value::from(1),
            "test".to_string(),
            Some(serde_json::json!({"a":1})),
        );
        let frame = encode_frame(&req).unwrap();
        let decoded: RpcRequest = decode_frame(&frame).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn roundtrip_response_result() {
        let resp = RpcResponse::new(
            Value::from(1),
            RpcResponsePayload::Result(serde_json::json!({"ok": true})),
        );
        let frame = encode_frame(&resp).unwrap();
        let decoded: RpcResponse = decode_frame(&frame).unwrap();
        assert_eq!(resp, decoded);
    }

    #[test]
    fn roundtrip_response_error() {
        let resp = RpcResponse::new(
            Value::from(2),
            RpcResponsePayload::Error(RpcError {
                code: -32600,
                message: "Invalid Request".to_string(),
                data: None,
            }),
        );
        let frame = encode_frame(&resp).unwrap();
        let decoded: RpcResponse = decode_frame(&frame).unwrap();
        assert_eq!(resp, decoded);
    }

    #[test]
    fn decode_truncated() {
        let req = RpcRequest::new(Value::from(1), "test".to_string(), None);
        let mut frame = encode_frame(&req).unwrap();
        frame.pop(); // remove last byte
        assert!(matches!(decode_frame::<RpcRequest>(&frame), Err(FrameError::Truncated)));
    }

    #[test]
    fn decode_malformed() {
        let mut frame = Vec::new();
        let payload = b"not json";
        frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        frame.extend_from_slice(payload);
        assert!(matches!(decode_frame::<RpcRequest>(&frame), Err(FrameError::Malformed(_))));
    }

    #[test]
    fn decode_truncated_header() {
        assert!(matches!(decode_frame::<RpcRequest>(&[1, 2, 3]), Err(FrameError::Truncated)));
    }

    #[test]
    fn encode_payload_too_large() {
        let big_data = vec![0u8; MAX_PAYLOAD_SIZE + 1];
        assert!(matches!(encode_frame(&big_data), Err(EncodeError::PayloadTooLarge)));
    }

    #[test]
    fn decode_payload_too_large() {
        let len = (MAX_PAYLOAD_SIZE + 1) as u32;
        let frame = len.to_le_bytes();
        assert!(matches!(decode_frame::<RpcRequest>(&frame), Err(FrameError::PayloadTooLarge)));
    }
}
