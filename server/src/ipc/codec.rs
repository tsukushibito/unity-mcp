use bytes::Bytes;
use prost::Message;
use thiserror::Error;

use crate::generated::mcp::unity::v1 as pb;

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

// Optional: calculate a schema hash (TODO: wire to handshake)
pub fn schema_hash() -> String {
    // For now, a constant or build-time string. Replace with descriptor-set hash if desired.
    "schema-v1".to_string()
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
        assert_eq!(hash, "schema-v1");
    }
}