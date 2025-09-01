// Unity MCP Bridge - TCP Transport
// Implements TCP server transport for IPC communication (Unix/Linux fallback)
using System;
using System.Net;
using System.Net.Sockets;
using System.Threading;
using System.Threading.Tasks;
using UnityEngine;

namespace Mcp.Unity.V1.Ipc
{
    internal sealed class TcpTransport : IDisposable
    {
        private readonly IPEndPoint _endpoint;
        private TcpListener _listener;
        private bool _disposed = false;

        public TcpTransport(IPEndPoint endpoint)
        {
            _endpoint = endpoint ?? throw new ArgumentNullException(nameof(endpoint));
        }

        /// <summary>
        /// Create TcpTransport with default loopback endpoint (127.0.0.1:7777)
        /// </summary>
        public static TcpTransport CreateDefault()
        {
            return new TcpTransport(new IPEndPoint(IPAddress.Loopback, 7777));
        }

        /// <summary>
        /// Create TcpTransport with specified port on loopback address (127.0.0.1)
        /// </summary>
        /// <param name="port">Port number to use</param>
        /// <returns>TcpTransport configured for the specified port</returns>
        public static TcpTransport CreateWithPort(int port)
        {
            return new TcpTransport(new IPEndPoint(IPAddress.Loopback, port));
        }

        /// <summary>
        /// Start the TCP listener
        /// </summary>
        public void Start()
        {
            if (_disposed) throw new ObjectDisposedException(nameof(TcpTransport));
            if (_listener != null) throw new InvalidOperationException("Transport already started");

            try
            {
                _listener = new TcpListener(_endpoint);
                _listener.Start();
                Debug.Log($"[TcpTransport] Started listening on {_endpoint}");
            }
            catch (Exception ex)
            {
                Debug.LogError($"[TcpTransport] Failed to start listener on {_endpoint}: {ex.Message}");
                throw;
            }
        }

        /// <summary>
        /// Stop the TCP listener
        /// </summary>
        public void Stop()
        {
            if (_listener != null)
            {
                try
                {
                    _listener.Stop();
                    Debug.Log($"[TcpTransport] Stopped listening on {_endpoint}");
                }
                catch (Exception ex)
                {
                    Debug.LogWarning($"[TcpTransport] Error stopping listener: {ex.Message}");
                }
                finally
                {
                    _listener = null;
                }
            }
        }

        /// <summary>
        /// Accept a client connection and return the network stream
        /// </summary>
        public async Task<NetworkStream> AcceptAsync(CancellationToken cancellationToken = default)
        {
            if (_disposed) throw new ObjectDisposedException(nameof(TcpTransport));
            if (_listener == null) throw new InvalidOperationException("Transport not started");

            try
            {
                // Check cancellation before accepting connection
                cancellationToken.ThrowIfCancellationRequested();
                
                var tcpClient = await _listener.AcceptTcpClientAsync();
                var clientEndpoint = tcpClient.Client.RemoteEndPoint;
                Debug.Log($"[TcpTransport] Accepted connection from {clientEndpoint}");

                // Configure socket options for low latency
                tcpClient.NoDelay = true;
                tcpClient.Client.SetSocketOption(SocketOptionLevel.Socket, SocketOptionName.KeepAlive, true);

                return tcpClient.GetStream();
            }
            catch (ObjectDisposedException) when (_disposed)
            {
                throw new OperationCanceledException("Transport disposed");
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                Debug.Log("[TcpTransport] Accept operation was cancelled");
                throw;
            }
            catch (Exception ex)
            {
                Debug.LogError($"[TcpTransport] Error accepting connection: {ex.Message}");
                throw;
            }
        }

        /// <summary>
        /// Check if the transport is listening
        /// </summary>
        public bool IsListening => _listener != null && _listener.Server.IsBound;

        /// <summary>
        /// Get the configured endpoint (host and port)
        /// </summary>
        public IPEndPoint Endpoint => _endpoint;

        /// <summary>
        /// Get the configured port number
        /// </summary>
        public int Port => _endpoint.Port;

        public void Dispose()
        {
            if (!_disposed)
            {
                Stop();
                _disposed = true;
            }
        }
    }
}