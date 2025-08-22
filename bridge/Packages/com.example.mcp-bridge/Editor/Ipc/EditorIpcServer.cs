// Unity MCP Bridge - Editor IPC Server
// Main IPC server that handles handshake and Health requests
using System;
using System.Collections.Generic;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using UnityEditor;
using UnityEngine;
using Google.Protobuf;
using Pb = Mcp.Unity.V1;

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
        private static readonly Dictionary<Stream, Bridge.Editor.Ipc.FeatureGuard> _negotiatedFeatures = new();

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
                    // Step 1: Wait for handshake (T01: IpcControl with Hello)
                    var controlFrame = await Framing.ReadFrameAsync(stream);
                    if (controlFrame == null)
                    {
                        Debug.LogWarning("[EditorIpcServer] Connection closed before handshake");
                        return;
                    }

                    // T01: Decode as IpcControl directly
                    var control = IpcControl.Parser.ParseFrom(controlFrame);
                    if (control.Hello == null)
                    {
                        Debug.LogWarning("[EditorIpcServer] Invalid handshake: expected IpcControl.Hello");
                        await SendRejectAsync(stream, IpcReject.Types.Code.Internal, "Expected hello control message");
                        return;
                    }

                    var hello = control.Hello;
                    Debug.Log($"[EditorIpcServer] Received T01 handshake: version={hello.IpcVersion}, client={hello.ClientName}, features={string.Join(",", hello.Features)}");

                    // Phase 3: Comprehensive validation
                    // 1. Token validation
                    var tokenValidation = ValidateToken(hello.Token);
                    if (!tokenValidation.IsValid)
                    {
                        await SendRejectAsync(stream, tokenValidation.ErrorCode, tokenValidation.ErrorMessage);
                        return;
                    }

                    // 2. Version compatibility check
                    var versionValidation = ValidateVersion(hello.IpcVersion);
                    if (!versionValidation.IsValid)
                    {
                        await SendRejectAsync(stream, versionValidation.ErrorCode, versionValidation.ErrorMessage);
                        return;
                    }

                    // 3. Editor state validation
                    var editorValidation = ValidateEditorState();
                    if (!editorValidation.IsValid)
                    {
                        await SendRejectAsync(stream, editorValidation.ErrorCode, editorValidation.ErrorMessage);
                        return;
                    }

                    // 4. Project root validation
                    var pathValidation = ValidateProjectRoot(hello.ProjectRoot);
                    if (!pathValidation.IsValid)
                    {
                        await SendRejectAsync(stream, pathValidation.ErrorCode, pathValidation.ErrorMessage);
                        return;
                    }

                    // Step 2: Send T01 welcome response
                    await SendWelcomeAsync(stream, hello);
                    Debug.Log($"[EditorIpcServer] T01 Handshake completed: session={hello.ClientName}");

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
                        Bridge.Editor.Ipc.FeatureGuard features;
                        lock (_streamLock)
                        {
                            _negotiatedFeatures.TryGetValue(stream, out features);
                        }

                        if (features == null)
                        {
                            throw new InvalidOperationException("No negotiated features found for connection");
                        }

                        assetsResponse = AssetsHandler.Handle(request, features);
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
                        Bridge.Editor.Ipc.FeatureGuard features;
                        lock (_streamLock)
                        {
                            _negotiatedFeatures.TryGetValue(stream, out features);
                        }

                        if (features == null)
                        {
                            throw new InvalidOperationException("No negotiated features found for connection");
                        }

                        buildResponse = BuildHandler.Handle(request, features);
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
        /// Send T01 welcome response
        /// </summary>
        private static async Task SendWelcomeAsync(Stream stream, IpcHello hello)
        {
            var welcome = CreateWelcome(hello);

            // Store negotiated features for this connection
            lock (_streamLock)
            {
                _negotiatedFeatures[stream] = new Bridge.Editor.Ipc.FeatureGuard(welcome.AcceptedFeatures);
            }

            var welcomeControl = new IpcControl { Welcome = welcome };
            await SendControlFrameAsync(stream, welcomeControl);
        }

        /// <summary>
        /// Create welcome response with feature negotiation
        /// </summary>
        private static IpcWelcome CreateWelcome(IpcHello hello)
        {
            var clientFeatures = hello.Features;
            var serverFeatures = Bridge.Editor.Ipc.ServerFeatureConfig.GetEnabledFeatures();

            // Negotiate features - intersection of client and server capabilities
            var acceptedFeatures = Bridge.Editor.Ipc.FeatureFlagExtensions.NegotiateFeatures(clientFeatures);

            Debug.Log($"[EditorIpcServer] Feature negotiation: client requested {clientFeatures.Count}, " +
                      $"server supports {serverFeatures.Count}, accepted {acceptedFeatures.Count}");

            return new IpcWelcome
            {
                IpcVersion = hello.IpcVersion,
                AcceptedFeatures = { acceptedFeatures },
                SchemaHash = hello.SchemaHash, // Will be implemented in Phase 5
                ServerName = "unity-editor-bridge",
                ServerVersion = GetPackageVersion(),
                EditorVersion = Application.unityVersion,
                SessionId = Guid.NewGuid().ToString(),
                Meta = { { "platform", Application.platform.ToString() } }
            };
        }

        /// <summary>
        /// Get package version
        /// </summary>
        private static string GetPackageVersion()
        {
            // TODO: Get actual package version from package.json
            return "0.1.0";
        }

        /// <summary>
        /// Send T01 reject response
        /// </summary>
        private static async Task SendRejectAsync(Stream stream, IpcReject.Types.Code code, string message)
        {
            var reject = new IpcReject { Code = code, Message = message };
            var rejectControl = new IpcControl { Reject = reject };
            await SendControlFrameAsync(stream, rejectControl);
            Debug.LogWarning($"[EditorIpcServer] Sent reject response: {code} - {message}");
        }

        /// <summary>
        /// Send error response (for regular requests)
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
        /// Send T01 control frame
        /// </summary>
        private static async Task SendControlFrameAsync(Stream stream, IpcControl control)
        {
            var bytes = control.ToByteArray();
            await Framing.WriteFrameAsync(stream, bytes);
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
        /// Validation result structure
        /// </summary>
        private struct ValidationResult
        {
            public bool IsValid { get; }
            public IpcReject.Types.Code ErrorCode { get; }
            public string ErrorMessage { get; }

            private ValidationResult(bool isValid, IpcReject.Types.Code errorCode, string errorMessage)
            {
                IsValid = isValid;
                ErrorCode = errorCode;
                ErrorMessage = errorMessage;
            }

            public static ValidationResult Success() => new ValidationResult(true, default, null);
            public static ValidationResult Error(IpcReject.Types.Code code, string message) =>
                new ValidationResult(false, code, message);
        }

        /// <summary>
        /// Validate authentication token
        /// </summary>
        private static ValidationResult ValidateToken(string token)
        {
            // Check if token is empty
            if (string.IsNullOrEmpty(token))
            {
                return ValidationResult.Error(IpcReject.Types.Code.Unauthenticated, "missing token");
            }

            // Get expected token from configuration
            var expectedToken = GetConfiguredToken();
            if (string.IsNullOrEmpty(expectedToken))
            {
                // Development mode - accept any non-empty token
                return ValidationResult.Success();
            }

            // Production mode - exact match required
            if (token != expectedToken)
            {
                return ValidationResult.Error(IpcReject.Types.Code.Unauthenticated, "invalid token");
            }

            return ValidationResult.Success();
        }

        /// <summary>
        /// Validate IPC version compatibility
        /// </summary>
        private static ValidationResult ValidateVersion(string clientVersion)
        {
            const string ServerVersion = "1.0"; // Current server version

            if (string.IsNullOrEmpty(clientVersion))
            {
                return ValidationResult.Error(IpcReject.Types.Code.FailedPrecondition, "missing ipc_version");
            }

            // Parse major.minor
            var clientParts = clientVersion.Split('.');
            var serverParts = ServerVersion.Split('.');

            if (clientParts.Length < 2 || serverParts.Length < 2)
            {
                return ValidationResult.Error(IpcReject.Types.Code.OutOfRange, "invalid version format");
            }

            if (!int.TryParse(clientParts[0], out int clientMajor) ||
                !int.TryParse(serverParts[0], out int serverMajor))
            {
                return ValidationResult.Error(IpcReject.Types.Code.OutOfRange, "invalid version numbers");
            }

            // Major version must match exactly
            if (clientMajor != serverMajor)
            {
                return ValidationResult.Error(
                    IpcReject.Types.Code.OutOfRange,
                    $"ipc_version {clientVersion} not supported; server={ServerVersion}"
                );
            }

            return ValidationResult.Success();
        }

        /// <summary>
        /// Validate Unity Editor state
        /// </summary>
        private static ValidationResult ValidateEditorState()
        {
            // Check if Unity Editor is in a valid state
            if (EditorApplication.isCompiling)
            {
                return ValidationResult.Error(IpcReject.Types.Code.Unavailable, "editor compiling");
            }

            if (EditorApplication.isUpdating)
            {
                return ValidationResult.Error(IpcReject.Types.Code.Unavailable, "editor updating");
            }

            return ValidationResult.Success();
        }

        /// <summary>
        /// Validate project root path
        /// </summary>
        private static ValidationResult ValidateProjectRoot(string projectRoot)
        {
            if (string.IsNullOrEmpty(projectRoot))
            {
                return ValidationResult.Error(IpcReject.Types.Code.FailedPrecondition, "missing project_root");
            }

            try
            {
                var normalizedRoot = Path.GetFullPath(projectRoot);
                var actualProjectPath = Path.GetFullPath(Directory.GetCurrentDirectory());

                if (!normalizedRoot.Equals(actualProjectPath, StringComparison.OrdinalIgnoreCase))
                {
                    return ValidationResult.Error(
                        IpcReject.Types.Code.FailedPrecondition,
                        "project_root mismatch"
                    );
                }
            }
            catch (Exception ex)
            {
                Debug.LogException(ex);
                return ValidationResult.Error(
                    IpcReject.Types.Code.FailedPrecondition,
                    "invalid project_root path"
                );
            }

            return ValidationResult.Success();
        }

        /// <summary>
        /// Get configured authentication token
        /// </summary>
        private static string GetConfiguredToken()
        {
            // Try environment variable first
            var envToken = Environment.GetEnvironmentVariable("MCP_IPC_TOKEN");
            if (!string.IsNullOrEmpty(envToken))
            {
                return envToken;
            }

            // Try EditorPrefs
            var prefKey = "MCP.IpcToken";
            if (EditorPrefs.HasKey(prefKey))
            {
                return EditorPrefs.GetString(prefKey);
            }

            // No token configured - development mode
            return null;
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
                _negotiatedFeatures.Remove(stream);
                Debug.Log($"[EditorIpcServer] Unregistered stream, active count: {_activeStreams.Count}");
            }
        }
    }
}