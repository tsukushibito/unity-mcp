// Unity MCP Bridge - IPC Framing
// Implements 4-byte big-endian length prefix framing compatible with Rust LengthDelimitedCodec
using System;
using System.Buffers;
using System.IO;
using System.Threading.Tasks;

namespace Mcp.Unity.V1.Ipc
{
    internal static class Framing
    {
        private const int MaxFrameSize = 64 * 1024 * 1024; // 64MB max frame size

        /// <summary>
        /// Write a frame with 4-byte big-endian length prefix followed by payload
        /// </summary>
        public static async Task WriteFrameAsync(Stream stream, ReadOnlyMemory<byte> payload)
        {
            if (stream == null) throw new ArgumentNullException(nameof(stream));
            if (payload.Length > MaxFrameSize) throw new ArgumentException($"Frame too large: {payload.Length} > {MaxFrameSize}");

            // Write 4-byte big-endian length header
            Span<byte> lengthHeader = stackalloc byte[4];
            var length = payload.Length;
            lengthHeader[0] = (byte)((length >> 24) & 0xFF);
            lengthHeader[1] = (byte)((length >> 16) & 0xFF);
            lengthHeader[2] = (byte)((length >> 8) & 0xFF);
            lengthHeader[3] = (byte)(length & 0xFF);

            await stream.WriteAsync(lengthHeader);
            await stream.WriteAsync(payload);
            await stream.FlushAsync();
        }

        /// <summary>
        /// Read a frame with 4-byte big-endian length prefix
        /// </summary>
        /// <returns>Frame payload or null if stream closed</returns>
        public static async Task<byte[]?> ReadFrameAsync(Stream stream)
        {
            if (stream == null) throw new ArgumentNullException(nameof(stream));

            // Read 4-byte length header
            byte[] lengthHeader = new byte[4];
            int bytesRead = await ReadExactAsync(stream, lengthHeader, 0, 4);
            if (bytesRead == 0) return null; // Stream closed
            if (bytesRead < 4) throw new IOException("Unexpected end of stream while reading length header");

            // Parse big-endian length
            int length = (lengthHeader[0] << 24) | (lengthHeader[1] << 16) | (lengthHeader[2] << 8) | lengthHeader[3];
            if (length < 0 || length > MaxFrameSize) 
                throw new IOException($"Invalid frame length: {length} (max: {MaxFrameSize})");

            if (length == 0) return new byte[0];

            // Read payload using ArrayPool for efficiency
            byte[] rentedBuffer = ArrayPool<byte>.Shared.Rent(length);
            try
            {
                bytesRead = await ReadExactAsync(stream, rentedBuffer, 0, length);
                if (bytesRead < length) throw new IOException("Unexpected end of stream while reading payload");

                // Copy to result array
                byte[] result = new byte[length];
                Buffer.BlockCopy(rentedBuffer, 0, result, 0, length);
                return result;
            }
            finally
            {
                ArrayPool<byte>.Shared.Return(rentedBuffer);
            }
        }

        /// <summary>
        /// Read exactly the specified number of bytes from stream
        /// </summary>
        private static async Task<int> ReadExactAsync(Stream stream, byte[] buffer, int offset, int count)
        {
            int totalRead = 0;
            while (totalRead < count)
            {
                int bytesRead = await stream.ReadAsync(buffer, offset + totalRead, count - totalRead);
                if (bytesRead == 0) break; // End of stream
                totalRead += bytesRead;
            }
            return totalRead;
        }
    }
}