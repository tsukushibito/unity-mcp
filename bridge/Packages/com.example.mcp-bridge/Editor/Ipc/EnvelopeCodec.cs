// Unity MCP Bridge - IPC Envelope Codec
// Provides serialization/deserialization for IpcEnvelope messages
using Google.Protobuf;

namespace Mcp.Unity.V1.Ipc
{
    internal static class EnvelopeCodec
    {
        /// <summary>
        /// Encode IpcEnvelope to byte array
        /// </summary>
        public static byte[] Encode(IpcEnvelope envelope)
        {
            if (envelope == null) throw new System.ArgumentNullException(nameof(envelope));
            return envelope.ToByteArray();
        }

        /// <summary>
        /// Decode byte array to IpcEnvelope
        /// </summary>
        public static IpcEnvelope Decode(byte[] bytes)
        {
            if (bytes == null) throw new System.ArgumentNullException(nameof(bytes));
            return IpcEnvelope.Parser.ParseFrom(bytes);
        }

        /// <summary>
        /// Decode byte array segment to IpcEnvelope
        /// </summary>
        public static IpcEnvelope Decode(byte[] bytes, int offset, int length)
        {
            if (bytes == null) throw new System.ArgumentNullException(nameof(bytes));
            if (offset < 0 || offset >= bytes.Length) throw new System.ArgumentOutOfRangeException(nameof(offset));
            if (length < 0 || offset + length > bytes.Length) throw new System.ArgumentOutOfRangeException(nameof(length));
            
            return IpcEnvelope.Parser.ParseFrom(bytes, offset, length);
        }

        /// <summary>
        /// Create a request envelope with correlation ID
        /// </summary>
        public static IpcEnvelope CreateRequest(string correlationId, IpcRequest request)
        {
            return new IpcEnvelope
            {
                CorrelationId = correlationId ?? string.Empty,
                Request = request
            };
        }

        /// <summary>
        /// Create a response envelope with correlation ID
        /// </summary>
        public static IpcEnvelope CreateResponse(string correlationId, IpcResponse response)
        {
            return new IpcEnvelope
            {
                CorrelationId = correlationId ?? string.Empty,
                Response = response
            };
        }

        /// <summary>
        /// Create an event envelope (no correlation ID needed for events)
        /// </summary>
        public static IpcEnvelope CreateEvent(IpcEvent eventMessage)
        {
            return new IpcEnvelope
            {
                CorrelationId = string.Empty,
                Event = eventMessage
            };
        }
    }
}