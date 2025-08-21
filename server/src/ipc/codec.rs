use bytes::Bytes;
use prost::Message;
use thiserror::Error;

use crate::generated::mcp::unity::v1 as pb;
use crate::generated::schema_hash::SCHEMA_HASH;

#[derive(Debug, Error)]
pub enum CodecError {
    #[error("encode error: {0}")]
    Encode(#[from] prost::EncodeError),
    #[error("decode error: {0}")]
    Decode(#[from] prost::DecodeError),
}

pub fn encode_envelope(env: &pb::IpcEnvelope) -> Result<Bytes, CodecError> {
    let mut buf = bytes::BytesMut::with_capacity(env.encoded_len());
    env.encode(&mut buf)?;
    Ok(buf.freeze())
}

pub fn decode_envelope(b: Bytes) -> Result<pb::IpcEnvelope, CodecError> {
    pb::IpcEnvelope::decode(b).map_err(CodecError::Decode)
}

pub fn encode_control(control: &pb::IpcControl) -> Result<Bytes, CodecError> {
    let mut buf = bytes::BytesMut::with_capacity(control.encoded_len());
    control.encode(&mut buf)?;
    Ok(buf.freeze())
}

pub fn decode_control(b: Bytes) -> Result<pb::IpcControl, CodecError> {
    pb::IpcControl::decode(b).map_err(CodecError::Decode)
}

// Schema hash from build-time generated constant
pub fn schema_hash() -> Vec<u8> {
    SCHEMA_HASH.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let original = pb::IpcEnvelope {
            correlation_id: "test-123".to_string(),
            kind: Some(pb::ipc_envelope::Kind::Request(pb::IpcRequest {
                payload: Some(pb::ipc_request::Payload::Health(pb::HealthRequest {})),
            })),
        };

        let encoded = encode_envelope(&original).expect("encoding should succeed");
        let decoded = decode_envelope(encoded).expect("decoding should succeed");

        assert_eq!(original.correlation_id, decoded.correlation_id);
        match (original.kind, decoded.kind) {
            (
                Some(pb::ipc_envelope::Kind::Request(req1)),
                Some(pb::ipc_envelope::Kind::Request(req2)),
            ) => {
                match (req1.payload, req2.payload) {
                    (
                        Some(pb::ipc_request::Payload::Health(_)),
                        Some(pb::ipc_request::Payload::Health(_)),
                    ) => {} // Success
                    _ => panic!("Payload mismatch"),
                }
            }
            _ => panic!("Kind mismatch"),
        }
    }

    #[test]
    fn test_schema_hash() {
        let hash = schema_hash();
        assert_eq!(hash.len(), 32); // SHA-256 produces 32 bytes
    }

    #[test]
    fn test_ipc_control_roundtrip() {
        let hello = pb::IpcHello {
            token: "test-token".to_string(),
            ipc_version: "1.0".to_string(),
            features: vec!["assets.basic".to_string()],
            schema_hash: vec![1, 2, 3, 4],
            project_root: "/test/path".to_string(),
            client_name: "test-client".to_string(),
            client_version: "0.1.0".to_string(),
            meta: std::collections::HashMap::new(),
        };
        
        let control = pb::IpcControl {
            kind: Some(pb::ipc_control::Kind::Hello(hello.clone())),
        };

        let encoded = encode_control(&control).expect("encoding should succeed");
        let decoded = decode_control(encoded).expect("decoding should succeed");

        match decoded.kind {
            Some(pb::ipc_control::Kind::Hello(decoded_hello)) => {
                assert_eq!(hello.token, decoded_hello.token);
                assert_eq!(hello.ipc_version, decoded_hello.ipc_version);
                assert_eq!(hello.features, decoded_hello.features);
                assert_eq!(hello.schema_hash, decoded_hello.schema_hash);
                assert_eq!(hello.project_root, decoded_hello.project_root);
                assert_eq!(hello.client_name, decoded_hello.client_name);
                assert_eq!(hello.client_version, decoded_hello.client_version);
            }
            _ => panic!("Expected Hello variant"),
        }
    }
}