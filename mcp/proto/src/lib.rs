use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
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

pub fn encode_frame<T: Serialize>(message: &T) -> Result<Vec<u8>, EncodeError> {
    let payload = serde_json::to_vec(message)?;
    let len = u32::try_from(payload.len()).map_err(|_| EncodeError::PayloadTooLarge)?;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

pub fn decode_frame<T: for<'de> Deserialize<'de>>(data: &[u8]) -> Result<T, FrameError> {
    let header = data.get(0..4).ok_or(FrameError::Truncated)?;
    let len_bytes: [u8; 4] = header
        .try_into()
        .expect("infallible: slice is 4 bytes");
    let len = u32::from_le_bytes(len_bytes) as usize;
    let payload = data.get(4..4 + len).ok_or(FrameError::Truncated)?;
    Ok(serde_json::from_slice(payload)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_request() {
        let req = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Value::from(1),
            method: "test".to_string(),
            params: Some(serde_json::json!({"a":1})),
        };
        let frame = encode_frame(&req).unwrap();
        let decoded: RpcRequest = decode_frame(&frame).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn decode_truncated() {
        let req = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Value::from(1),
            method: "test".to_string(),
            params: None,
        };
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
}
