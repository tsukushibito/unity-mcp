// Unity MCP Bridge - Editor IPC Server
// Main IPC server that handles handshake and Health requests
using System;
using System.Collections.Generic;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using UnityEditor;
using UnityEngine;

namespace Mcp.Unity.V1.Ipc
{
    [InitializeOnLoad]
    internal static class EditorIpcServer
    {
        private static CancellationTokenSource _cancellationTokenSource;
        private static TcpTransport _transport;
        private static bool _isRunning = false;
        private static readonly List<Stream> _activeStreams = new();
        private static readonly object _streamLock = new();

        static EditorIpcServer()
        {
            Debug.Log("[EditorIpcServer] Initializing IPC server...");
            
            // Start the server automatically when Unity Editor loads
            _ = StartAsync();

            // Clean shutdown when Unity Editor closes
            EditorApplication.quitting += Shutdown;
        }

        /// <summary>
        /// Start the IPC server
        /// </summary>
        public static async Task StartAsync()
        {
            if (_isRunning)
            {
                Debug.LogWarning("[EditorIpcServer] Server is already running");
                return;
            }

            try
            {
                // Cancel any existing operation
                _cancellationTokenSource?.Cancel();
                _cancellationTokenSource = new CancellationTokenSource();
                
                _transport = TcpTransport.CreateDefault();
                _transport.Start();
                _isRunning = true;

                Debug.Log("[EditorIpcServer] IPC server started successfully");

                // Start accepting connections in background
                await Task.Run(() => AcceptConnectionsAsync(_cancellationTokenSource.Token));
            }
            catch (Exception ex)
            {
                Debug.LogError($"[EditorIpcServer] Failed to start IPC server: {ex.Message}");
                _isRunning = false;
                throw;
            }
        }

        /// <summary>
        /// Stop the IPC server
        /// </summary>
        public static void Shutdown()
        {
            if (!_isRunning) return;

            Debug.Log("[EditorIpcServer] Shutting down IPC server...");

            try
            {
                _cancellationTokenSource?.Cancel();
                _transport?.Stop();
                _transport?.Dispose();
            }
            catch (Exception ex)
            {
                Debug.LogWarning($"[EditorIpcServer] Error during shutdown: {ex.Message}");
            }
            finally
            {
                _cancellationTokenSource = null;
                _transport = null;
                _isRunning = false;
                Debug.Log("[EditorIpcServer] IPC server stopped");
            }
        }

        /// <summary>
        /// Accept incoming connections loop
        /// </summary>
        private static async Task AcceptConnectionsAsync(CancellationToken cancellationToken)
        {
            if (_transport == null) return;

            Debug.Log("[EditorIpcServer] Starting connection acceptance loop");

            try
            {
                while (!cancellationToken.IsCancellationRequested && _transport.IsListening)
                {
                    try
                    {
                        var stream = await _transport.AcceptAsync(cancellationToken);
                        // Handle each connection in its own task to allow concurrent connections
                        _ = Task.Run(() => HandleConnectionAsync(stream, cancellationToken), cancellationToken);
                    }
                    catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
                    {
                        break;
                    }
                    catch (Exception ex)
                    {
                        Debug.LogError($"[EditorIpcServer] Error accepting connection: {ex.Message}");
                        // Continue accepting other connections
                        await Task.Delay(100, cancellationToken);
                    }
                }
            }
            catch (OperationCanceledException)
            {
                // Expected when cancellation is requested
            }
            catch (Exception ex)
            {
                Debug.LogError($"[EditorIpcServer] Connection acceptance loop failed: {ex.Message}");
            }

            Debug.Log("[EditorIpcServer] Connection acceptance loop ended");
        }

        /// <summary>
        /// Handle a single IPC connection
        /// </summary>
        private static async Task HandleConnectionAsync(Stream stream, CancellationToken cancellationToken)
        {
            Debug.Log("[EditorIpcServer] Handling new connection");

            try
            {
                using (stream)
                {
                    // Step 1: Wait for handshake (IpcHello wrapped in IpcRequest)
                    var helloFrame = await Framing.ReadFrameAsync(stream);
                    if (helloFrame == null)
                    {
                        Debug.LogWarning("[EditorIpcServer] Connection closed before handshake");
                        return;
                    }

                    var helloEnvelope = EnvelopeCodec.Decode(helloFrame);
                    if (helloEnvelope.Request?.Hello == null)
                    {
                        Debug.LogWarning("[EditorIpcServer] Invalid handshake: expected IpcHello");
                        await SendErrorAsync(stream, helloEnvelope.CorrelationId, 400, "Expected hello message");
                        return;
                    }

                    var hello = helloEnvelope.Request.Hello;
                    Debug.Log($"[EditorIpcServer] Received handshake: version={hello.IpcVersion}, schema={hello.SchemaHash}");

                    // TODO: Validate token and schema_hash if needed
                    // For now, accept all connections

                    // Step 2: Send welcome response
                    await SendWelcomeAsync(stream, helloEnvelope.CorrelationId);
                    Debug.Log("[EditorIpcServer] Handshake completed");

                    // Step 3: Register the stream as active
                    RegisterStream(stream);

                    // Step 4: Enter request processing loop
                    await ProcessRequestsAsync(stream, cancellationToken);
                }
            }
            catch (Exception ex)
            {
                Debug.LogError($"[EditorIpcServer] Connection handling failed: {ex.Message}");
            }
            finally
            {
                // Step 5: Unregister the stream
                UnregisterStream(stream);
            }
        }

        /// <summary>
        /// Process requests from an established IPC connection
        /// </summary>
        private static async Task ProcessRequestsAsync(Stream stream, CancellationToken cancellationToken)
        {
            Debug.Log("[EditorIpcServer] Starting request processing loop");

            try
            {
                while (!cancellationToken.IsCancellationRequested)
                {
                    var frame = await Framing.ReadFrameAsync(stream);
                    if (frame == null) break; // Connection closed

                    var envelope = EnvelopeCodec.Decode(frame);
                    if (envelope.Request == null)
                    {
                        Debug.LogWarning("[EditorIpcServer] Received non-request message, ignoring");
                        continue;
                    }

                    await DispatchRequestAsync(stream, envelope.CorrelationId, envelope.Request);
                }
            }
            catch (Exception ex)
            {
                Debug.LogError($"[EditorIpcServer] Request processing failed: {ex.Message}");
            }

            Debug.Log("[EditorIpcServer] Request processing loop ended");
        }

        /// <summary>
        /// Dispatch a request to the appropriate handler
        /// </summary>
        private static async Task DispatchRequestAsync(Stream stream, string correlationId, IpcRequest request)
        {
            try
            {
                switch (request.PayloadCase)
                {
                    case IpcRequest.PayloadOneofCase.Health:
                        await HandleHealthRequest(stream, correlationId, request.Health);
                        break;

                    case IpcRequest.PayloadOneofCase.Assets:
                        await HandleAssetsRequest(stream, correlationId, request.Assets);
                        break;

                    case IpcRequest.PayloadOneofCase.Build:
                        await HandleBuildRequest(stream, correlationId, request.Build);
                        break;

                    case IpcRequest.PayloadOneofCase.Hello:
                        Debug.LogWarning("[EditorIpcServer] Received hello after handshake, ignoring");
                        break;

                    default:
                        Debug.LogWarning($"[EditorIpcServer] Unhandled request type: {request.PayloadCase}");
                        await SendErrorAsync(stream, correlationId, 404, $"Unhandled request: {request.PayloadCase}");
                        break;
                }
            }
            catch (Exception ex)
            {
                Debug.LogError($"[EditorIpcServer] Request dispatch failed: {ex.Message}");
                await SendErrorAsync(stream, correlationId, 500, "Internal server error");
            }
        }

        /// <summary>
        /// Handle Health request
        /// </summary>
        private static async Task HandleHealthRequest(Stream stream, string correlationId, HealthRequest request)
        {
            Debug.Log("[EditorIpcServer] Processing health request");

            // Get Unity state information
            var ready = !EditorApplication.isCompiling && !EditorApplication.isUpdating;
            var version = Application.unityVersion;
            var status = ready ? "OK" : "BUSY";

            var healthResponse = new HealthResponse
            {
                Ready = ready,
                Version = version,
                Status = status
            };

            var response = new IpcResponse
            {
                CorrelationId = correlationId,
                Health = healthResponse
            };

            await SendResponseAsync(stream, response);
            Debug.Log($"[EditorIpcServer] Sent health response: ready={ready}, version={version}");
        }

        /// <summary>
        /// Handle Assets request
        /// </summary>
        private static async Task HandleAssetsRequest(Stream stream, string correlationId, AssetsRequest request)
        {
            Debug.Log($"[EditorIpcServer] Processing assets request: {request.PayloadCase}");

            // Assets operations must run on the main thread
            AssetsResponse assetsResponse = null;
            await Task.Run(() =>
            {
                // Use EditorApplication.delayCall to marshal to main thread
                var tcs = new TaskCompletionSource<AssetsResponse>();
                EditorApplication.delayCall += () =>
                {
                    try
                    {
                        assetsResponse = AssetsHandler.Handle(request);
                        tcs.SetResult(assetsResponse);
                    }
                    catch (Exception ex)
                    {
                        tcs.SetException(ex);
                    }
                };
                return tcs.Task;
            });

            var response = new IpcResponse
            {
                CorrelationId = correlationId,
                Assets = assetsResponse
            };

            await SendResponseAsync(stream, response);
            Debug.Log($"[EditorIpcServer] Sent assets response: status={assetsResponse.StatusCode}");
        }

        /// <summary>
        /// Handle Build request
        /// </summary>
        private static async Task HandleBuildRequest(Stream stream, string correlationId, BuildRequest request)
        {
            Debug.Log($"[EditorIpcServer] Processing build request: {request.PayloadCase}");

            // Build operations must run on the main thread
            BuildResponse buildResponse = null;
            await Task.Run(() =>
            {
                // Use EditorApplication.delayCall to marshal to main thread
                var tcs = new TaskCompletionSource<BuildResponse>();
                EditorApplication.delayCall += () =>
                {
                    try
                    {
                        buildResponse = BuildHandler.Handle(request);
                        tcs.SetResult(buildResponse);
                    }
                    catch (Exception ex)
                    {
                        tcs.SetException(ex);
                    }
                };
                return tcs.Task;
            });

            var response = new IpcResponse
            {
                CorrelationId = correlationId,
                Build = buildResponse
            };

            await SendResponseAsync(stream, response);
            Debug.Log($"[EditorIpcServer] Sent build response: status={buildResponse.Player?.StatusCode ?? buildResponse.Bundles?.StatusCode}");
        }

        /// <summary>
        /// Send welcome response
        /// </summary>
        private static async Task SendWelcomeAsync(Stream stream, string correlationId)
        {
            var welcome = new IpcWelcome
            {
                Ok = true,
                Error = string.Empty
            };

            var response = new IpcResponse
            {
                CorrelationId = correlationId,
                Welcome = welcome
            };

            await SendResponseAsync(stream, response);
        }

        /// <summary>
        /// Send error response
        /// </summary>
        private static async Task SendErrorAsync(Stream stream, string correlationId, int code, string message)
        {
            var response = new IpcResponse
            {
                CorrelationId = correlationId
                // Note: IpcResponse doesn't have status_code/message fields in current proto
                // Error information is conveyed through the welcome.error field during handshake
                // For other errors, we just send an empty response
            };

            await SendResponseAsync(stream, response);
            Debug.LogWarning($"[EditorIpcServer] Sent error response: {code} - {message}");
        }

        /// <summary>
        /// Send response envelope
        /// </summary>
        private static async Task SendResponseAsync(Stream stream, IpcResponse response)
        {
            var envelope = EnvelopeCodec.CreateResponse(response.CorrelationId, response);
            var bytes = EnvelopeCodec.Encode(envelope);
            await Framing.WriteFrameAsync(stream, bytes);
        }

        /// <summary>
        /// Get server status for debugging
        /// </summary>
        public static bool IsRunning => _isRunning;

        /// <summary>
        /// Try to get an active stream for event sending
        /// </summary>
        public static bool TryGetActiveStream(out Stream stream)
        {
            lock (_streamLock)
            {
                // Remove any closed streams
                for (int i = _activeStreams.Count - 1; i >= 0; i--)
                {
                    var s = _activeStreams[i];
                    if (!s.CanWrite)
                    {
                        _activeStreams.RemoveAt(i);
                        try { s.Dispose(); } catch { }
                    }
                }

                if (_activeStreams.Count > 0)
                {
                    stream = _activeStreams[0]; // Return first active stream
                    return true;
                }

                stream = null;
                return false;
            }
        }

        /// <summary>
        /// Register an active stream
        /// </summary>
        private static void RegisterStream(Stream stream)
        {
            lock (_streamLock)
            {
                _activeStreams.Add(stream);
                Debug.Log($"[EditorIpcServer] Registered stream, active count: {_activeStreams.Count}");
            }
        }

        /// <summary>
        /// Unregister a stream
        /// </summary>
        private static void UnregisterStream(Stream stream)
        {
            lock (_streamLock)
            {
                _activeStreams.Remove(stream);
                Debug.Log($"[EditorIpcServer] Unregistered stream, active count: {_activeStreams.Count}");
            }
        }
    }
}