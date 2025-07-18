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

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FrameError {
    #[error("frame too short")] 
    Truncated,
    #[error("invalid json")]
    Malformed,
}

pub fn encode_frame<T: Serialize>(message: &T) -> Result<Vec<u8>, serde_json::Error> {
    let payload = serde_json::to_vec(message)?;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

pub fn decode_frame<T: for<'de> Deserialize<'de>>(data: &[u8]) -> Result<T, FrameError> {
    if data.len() < 4 {
        return Err(FrameError::Truncated);
    }
    let len = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
    if data.len() < 4 + len {
        return Err(FrameError::Truncated);
    }
    let payload = &data[4..4 + len];
    serde_json::from_slice(payload).map_err(|_| FrameError::Malformed)
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
        assert_eq!(decode_frame::<RpcRequest>(&frame), Err(FrameError::Truncated));
    }

    #[test]
    fn decode_malformed() {
        let mut frame = Vec::new();
        let payload = b"not json";
        frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        frame.extend_from_slice(payload);
        assert_eq!(decode_frame::<RpcRequest>(&frame), Err(FrameError::Malformed));
    }
}
