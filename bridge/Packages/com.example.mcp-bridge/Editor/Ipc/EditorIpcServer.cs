// Unity MCP Bridge - Editor IPC Server
// Main IPC server that handles handshake and Health requests
using System;
using System.Collections.Generic;
using System.IO;
using System.Collections.Concurrent;
using System.Threading;
using System.Net.Sockets;
using System.Threading.Tasks;
using UnityEditor;
using UnityEngine;
using Google.Protobuf;
using Bridge.Editor.Ipc.Handlers;
using Pb = Mcp.Unity.V1;
using Mcp.Unity.V1.Ipc.Infra;
using Bridge.Editor.Ipc.Infra;

namespace Mcp.Unity.V1.Ipc
{
    [InitializeOnLoad]
    internal static class EditorIpcServer
    {
        private static CancellationTokenSource _cancellationTokenSource;
        private static TcpTransport _transport;
        private static Task _acceptLoopTask;
        private static bool _isRunning = false;
        private static readonly List<Stream> _activeStreams = new();
        private static readonly object _streamLock = new();
        private static readonly Dictionary<Stream, Bridge.Editor.Ipc.FeatureGuard> _negotiatedFeatures = new();
        private static readonly ConcurrentDictionary<Stream, SemaphoreSlim> _writeLocks = new();
        private static readonly Dictionary<Stream, TcpClient> _clientsByStream = new();
        private static string _cachedToken;
        private static int _cachedPort;

        static EditorIpcServer()
        {
            Debug.Log("[EditorIpcServer] Initializing IPC server...");

            // Defer Unity API access to main thread to avoid cross-thread violations
            EditorDispatcher.RunOnMainAsync(async () =>
            {
                try
                {
                    // Cache authentication token and port on main thread
                    _cachedToken = LoadTokenFromPrefs();
                    _cachedPort = LoadPortFromPrefs();

                    // Clean shutdown when Unity Editor closes
                    EditorApplication.quitting += Shutdown;

                    // Start the server automatically when Unity Editor loads
                    await StartAsync();

                    Debug.Log("[EditorIpcServer] Main thread initialization completed");
                }
                catch (System.Exception ex)
                {
                    Debug.LogError($"[EditorIpcServer] Main thread initialization failed: {ex}");
                }
            });
        }

        /// <summary>
        /// Start the IPC server
        /// </summary>
        public static Task StartAsync()
        {
            if (_isRunning)
            {
                Debug.LogWarning("[EditorIpcServer] Server is already running");
                return Task.CompletedTask;
            }

            try
            {
                _cancellationTokenSource?.Cancel();
                _cancellationTokenSource = new CancellationTokenSource();

                _transport = TcpTransport.CreateWithPort(_cachedPort);
                _transport.Start();
                _isRunning = true;

                _acceptLoopTask = Task.Run(() => AcceptConnectionsAsync(_cancellationTokenSource.Token));

                Debug.Log("[EditorIpcServer] IPC server started successfully");
            }
            catch (Exception ex)
            {
                Debug.LogError($"[EditorIpcServer] Failed to start IPC server: {ex.Message}");
                _isRunning = false;
                throw;
            }

            return Task.CompletedTask;
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

                lock (_streamLock)
                {
                    foreach (var kv in _clientsByStream)
                    {
                        try { kv.Value.Close(); kv.Value.Dispose(); } catch { }
                        try { kv.Key.Dispose(); } catch { }
                    }
                    _clientsByStream.Clear();
                    _activeStreams.Clear();
                    _negotiatedFeatures.Clear();
                    _writeLocks.Clear();
                }

                _transport?.Stop();
                _transport?.Dispose();

                try { _acceptLoopTask?.Wait(1000); } catch { }
            }
            catch (Exception ex)
            {
                Debug.LogWarning($"[EditorIpcServer] Error during shutdown: {ex.Message}");
            }
            finally
            {
                _acceptLoopTask = null;
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
                        var client = await _transport.AcceptAsync(cancellationToken);
                        if (cancellationToken.IsCancellationRequested)
                        {
                            client.Dispose();
                            break;
                        }
                        // Handle each connection in its own task to allow concurrent connections
                        // TODO(BG_ORIGIN): Task.Run creates background thread that may call Unity APIs
                        _ = Task.Run(() => HandleConnectionAsync(client, cancellationToken));
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
        /// <summary>
        /// Handle a single IPC connection
        /// </summary>
        private static async Task HandleConnectionAsync(TcpClient tcpClient, CancellationToken cancellationToken)
        {
            Debug.Log("[EditorIpcServer] Handling new connection");

            using (tcpClient)
            using (var stream = tcpClient.GetStream())
            {
                lock (_streamLock)
                {
                    _clientsByStream[stream] = tcpClient;
                }

                try
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

                    // Phase 1: BG-safe validations (Unity API non-dependent)
                    var versionValidation = ValidateVersion(hello.IpcVersion);
                    var schemaValidation = ValidateSchemaHash(hello.SchemaHash);

                    // Phase 2: Single main-thread block for Unity API access and final decision
                    var controlMessage = await EditorDispatcher.RunOnMainAsync(() =>
                    {
                        // Unity API: Get expected token on main thread
                        MainThreadGuard.AssertMainThread();
                        var expectedToken = UnityEditor.EditorUserSettings.GetConfigValue("MCP.IpcToken");

                        // BG-safe validation: Pure comparison (Unity API non-dependent)
                        var tokenValidation = ValidateToken(expectedToken, hello.Token);

                        // Early reject decisions
                        if (!tokenValidation.IsValid)
                            return new IpcControl { Reject = new IpcReject { Code = tokenValidation.ErrorCode, Message = tokenValidation.ErrorMessage } };

                        if (!versionValidation.IsValid)
                            return new IpcControl { Reject = new IpcReject { Code = versionValidation.ErrorCode, Message = versionValidation.ErrorMessage } };

                        if (!schemaValidation.IsValid)
                            return new IpcControl { Reject = new IpcReject { Code = schemaValidation.ErrorCode, Message = schemaValidation.ErrorMessage } };

                        // Unity API touches must be here
                        MainThreadGuard.AssertMainThread();

                        var editorValidation = ValidateEditorState(); // Synchronous, main-thread version
                        if (!editorValidation.IsValid)
                            return new IpcControl { Reject = new IpcReject { Code = editorValidation.ErrorCode, Message = editorValidation.ErrorMessage } };

                        var welcome = CreateWelcome(hello); // Synchronous, main-thread version
                        return new IpcControl { Welcome = welcome };
                    });

                    // Step 2: Send control response and handle result
                    if (controlMessage.Reject != null)
                    {
                        await SendControlFrameAsync(stream, controlMessage);
                        Debug.LogWarning($"[EditorIpcServer] Sent reject response: {controlMessage.Reject.Code} - {controlMessage.Reject.Message}");
                        return;
                    }

                    // Step 3: Send welcome and register features
                    await SendWelcomeAsync(stream, controlMessage.Welcome);
                    Debug.Log($"[EditorIpcServer] T01 Handshake completed: session={hello.ClientName}");

                    // Step 4: Register the stream as active
                    RegisterStream(stream);

                    // Step 5: Enter request processing loop
                    await ProcessRequestsAsync(stream, cancellationToken);
                }
                catch (Exception ex)
                {
                    Debug.LogError($"[EditorIpcServer] Connection handling failed: {ex.Message}");
                }
                finally
                {
                    UnregisterStream(stream);
                }
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
                    byte[] frame;
                    try
                    {
                        frame = await Framing.ReadFrameAsync(stream);
                    }
                    catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
                    {
                        // Normal shutdown
                        break;
                    }
                    catch (IOException)
                    {
                        // Treat IO errors during read as normal connection close
                        break;
                    }
                    catch (ObjectDisposedException)
                    {
                        // Stream disposed/closed by peer
                        break;
                    }
                    catch (SocketException)
                    {
                        // Connection reset by peer etc.
                        break;
                    }
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

                    case IpcRequest.PayloadOneofCase.GetCompileDiagnostics:
                        await HandleGetCompileDiagnosticsRequest(stream, correlationId, request.GetCompileDiagnostics);
                        break;

                    case IpcRequest.PayloadOneofCase.GetProjectSettings:
                        await HandleGetProjectSettingsRequest(stream, correlationId, request.GetProjectSettings);
                        break;

                    case IpcRequest.PayloadOneofCase.SetProjectSettings:
                        await HandleSetProjectSettingsRequest(stream, correlationId, request.SetProjectSettings);
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

            var response = await HealthHandler.HandleAsync(request);
            response.CorrelationId = correlationId;

            await SendResponseAsync(stream, response);
            Debug.Log($"[EditorIpcServer] Sent health response: ready={response.Health.Ready}, version={response.Health.Version}");
        }

        /// <summary>
        /// Handle GetCompileDiagnostics request
        /// </summary>
        private static async Task HandleGetCompileDiagnosticsRequest(Stream stream, string correlationId, GetCompileDiagnosticsRequest request)
        {
            Debug.Log($"[EditorIpcServer] Processing compile diagnostics request");

            // Diagnostics operations must run on the main thread to access Unity APIs
            var diagnosticsResponse = await EditorDispatcher.RunOnMainAsync(() =>
            {
                MainThreadGuard.AssertMainThread();
                return DiagnosticsHandler.Handle(request);
            });

            var response = new IpcResponse
            {
                CorrelationId = correlationId,
                GetCompileDiagnostics = diagnosticsResponse
            };

            await SendResponseAsync(stream, response);
            Debug.Log($"[EditorIpcServer] Sent diagnostics response: success={diagnosticsResponse.Success}, diagnostics_count={diagnosticsResponse.Diagnostics.Count}");
        }

        /// <summary>
        /// Handle GetProjectSettings request
        /// </summary>
        private static async Task HandleGetProjectSettingsRequest(Stream stream, string correlationId, GetProjectSettingsRequest request)
        {
            Debug.Log("[EditorIpcServer] Processing project settings get request");

            var settingsResponse = await EditorDispatcher.RunOnMainAsync(() =>
            {
                MainThreadGuard.AssertMainThread();
                return ProjectSettingsHandler.HandleGet(request);
            });

            var response = new IpcResponse
            {
                CorrelationId = correlationId,
                GetProjectSettings = settingsResponse
            };

            await SendResponseAsync(stream, response);
        }

        /// <summary>
        /// Handle SetProjectSettings request
        /// </summary>
        private static async Task HandleSetProjectSettingsRequest(Stream stream, string correlationId, SetProjectSettingsRequest request)
        {
            Debug.Log("[EditorIpcServer] Processing project settings set request");

            var setResponse = await EditorDispatcher.RunOnMainAsync(() =>
            {
                MainThreadGuard.AssertMainThread();
                return ProjectSettingsHandler.HandleSet(request);
            });

            var response = new IpcResponse
            {
                CorrelationId = correlationId,
                SetProjectSettings = setResponse
            };

            await SendResponseAsync(stream, response);
        }

        /// <summary>
        /// Handle Assets request
        /// </summary>
        private static async Task HandleAssetsRequest(Stream stream, string correlationId, AssetsRequest request)
        {
            Debug.Log($"[EditorIpcServer] Processing assets request: {request.PayloadCase}");

            // Assets operations must run on the main thread. Marshal via EditorDispatcher.
            var assetsResponse = await EditorDispatcher.RunOnMainAsync(() =>
            {
                MainThreadGuard.AssertMainThread();
                Bridge.Editor.Ipc.FeatureGuard features;
                lock (_streamLock)
                {
                    _negotiatedFeatures.TryGetValue(stream, out features);
                }

                if (features == null)
                {
                    throw new InvalidOperationException("No negotiated features found for connection");
                }

                return AssetsHandler.Handle(request, features);
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

            // Build operations must run on the main thread. Marshal via EditorDispatcher.
            var buildResponse = await EditorDispatcher.RunOnMainAsync(() =>
            {
                MainThreadGuard.AssertMainThread();
                Bridge.Editor.Ipc.FeatureGuard features;
                lock (_streamLock)
                {
                    _negotiatedFeatures.TryGetValue(stream, out features);
                }

                if (features == null)
                {
                    throw new InvalidOperationException("No negotiated features found for connection");
                }

                return BuildHandler.Handle(request, features);
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
        /// <summary>
        /// Send T01 welcome response
        /// </summary>
        private static async Task SendWelcomeAsync(Stream stream, IpcWelcome welcome)
        {
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
        /// <summary>
        /// Create welcome response with feature negotiation (must be called from main thread)
        /// </summary>
        private static IpcWelcome CreateWelcome(IpcHello hello)
        {
            MainThreadGuard.AssertMainThread();

            var clientFeatures = hello.Features;
            var serverFeatures = Bridge.Editor.Ipc.ServerFeatureConfig.GetEnabledFeatures();

            // Negotiate features - intersection of client and server capabilities
            var acceptedFeatures = Bridge.Editor.Ipc.FeatureFlagExtensions.NegotiateFeatures(clientFeatures);

            Debug.Log($"[EditorIpcServer] Feature negotiation: client requested {clientFeatures.Count}, " +
                      $"server supports {serverFeatures.Count}, accepted {acceptedFeatures.Count}");

            // Get Unity version and platform (safe on main thread)
#if UNITY_EDITOR && DEBUG
            Diag.LogUnityApiAccess("Application.unityVersion", "CreateWelcome");
            Diag.LogUnityApiAccess("Application.platform", "CreateWelcome");
#endif
            var unityVersion = Application.unityVersion;
            var platformString = Application.platform.ToString();

            return new IpcWelcome
            {
                IpcVersion = hello.IpcVersion,
                AcceptedFeatures = { acceptedFeatures },
                SchemaHash = Google.Protobuf.ByteString.CopyFrom(Mcp.Unity.V1.Generated.Schema.SchemaHashBytes),
                ServerName = "unity-editor-bridge",
                ServerVersion = GetPackageVersion(),
                EditorVersion = unityVersion,
                SessionId = Guid.NewGuid().ToString(),
                Meta = { { "platform", platformString } }
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
            await WriteFrameThreadSafe(stream, bytes);
        }

        /// <summary>
        /// Send response envelope
        /// </summary>
        private static async Task SendResponseAsync(Stream stream, IpcResponse response)
        {
            var envelope = EnvelopeCodec.CreateResponse(response.CorrelationId, response);
            var bytes = EnvelopeCodec.Encode(envelope);
            await WriteFrameThreadSafe(stream, bytes);
        }

        /// <summary>
        /// Ensure frames written to a given stream are serialized to avoid interleaving.
        /// </summary>
        internal static async Task WriteFrameThreadSafe(Stream stream, ReadOnlyMemory<byte> payload)
        {
            if (stream == null) throw new ArgumentNullException(nameof(stream));
            var gate = _writeLocks.GetOrAdd(stream, _ => new SemaphoreSlim(1, 1));
            await gate.WaitAsync().ConfigureAwait(false);
            try
            {
                await Framing.WriteFrameAsync(stream, payload).ConfigureAwait(false);
            }
            finally
            {
                gate.Release();
            }
        }

        /// <summary>
        /// Get server status for debugging
        /// </summary>
        public static bool IsRunning => _isRunning;
        /// <summary>
        /// Check if the IPC server is running and ready to accept connections
        /// </summary>
        public static bool IsReady => _isRunning && _transport?.IsListening == true;

        /// <summary>
        /// Get the current server port
        /// </summary>
        public static int CurrentPort => _transport?.Port ?? _cachedPort;
        /// <summary>
        /// Reload configuration from preferences (for testing purposes)
        /// </summary>
        public static void ReloadConfiguration()
        {
            if (!Application.isEditor) return;
            
            EditorDispatcher.RunOnMainAsync(() =>
            {
                _cachedToken = LoadTokenFromPrefs();
                _cachedPort = LoadPortFromPrefs();
                Debug.Log($"[EditorIpcServer] Configuration reloaded: port={_cachedPort}");
            });
        }

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
        private static ValidationResult ValidateToken(string expectedToken, string clientToken)
        {
            // A2-2: No development mode - always require token
            if (string.IsNullOrEmpty(expectedToken))
            {
                return ValidationResult.Error(IpcReject.Types.Code.Unauthenticated,
                    "Missing or empty token. Set EditorUserSettings: MCP.IpcToken");
            }

            // Check if client provided token is empty
            if (string.IsNullOrEmpty(clientToken))
            {
                return ValidationResult.Error(IpcReject.Types.Code.Unauthenticated,
                    "Missing or empty token. Set EditorUserSettings: MCP.IpcToken");
            }

            // Exact match required
            if (clientToken != expectedToken)
            {
                return ValidationResult.Error(IpcReject.Types.Code.Unauthenticated,
                    "Invalid token. Check EditorUserSettings: MCP.IpcToken");
            }

            return ValidationResult.Success();
        }

        /// <summary>
        /// Validate schema hash matches expected value
        /// </summary>
        private static ValidationResult ValidateSchemaHash(Google.Protobuf.ByteString clientSchemaHash)
        {
            var expectedHash = Mcp.Unity.V1.Generated.Schema.SchemaHashBytes;

            if (clientSchemaHash == null || clientSchemaHash.IsEmpty)
            {
                return ValidationResult.Error(IpcReject.Types.Code.FailedPrecondition,
                    "Schema hash missing. Regenerate C# SCHEMA_HASH from server (CI).");
            }

            var clientHashBytes = clientSchemaHash.ToByteArray();

            if (clientHashBytes.Length != expectedHash.Length)
            {
                return ValidationResult.Error(IpcReject.Types.Code.FailedPrecondition,
                    "Schema hash length mismatch. Regenerate C# SCHEMA_HASH from server (CI).");
            }

            for (int i = 0; i < expectedHash.Length; i++)
            {
                if (clientHashBytes[i] != expectedHash[i])
                {
                    return ValidationResult.Error(IpcReject.Types.Code.FailedPrecondition,
                        "Schema hash mismatch. Regenerate C# SCHEMA_HASH from server (CI).");
                }
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
        /// <summary>
        /// Validate Unity Editor state (must be called from main thread)
        /// </summary>
        private static ValidationResult ValidateEditorState()
        {
            MainThreadGuard.AssertMainThread();
#if UNITY_EDITOR && DEBUG
            Diag.LogUnityApiAccess("EditorApplication.isCompiling", "ValidateEditorState");
            Diag.LogUnityApiAccess("EditorApplication.isUpdating", "ValidateEditorState");
#endif

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
        /// Get configured authentication token
        /// </summary>
        private static string GetConfiguredToken()
        {
            return _cachedToken;
        }

        /// <summary>
        /// Load authentication token from preferences (called on main thread)
        /// </summary>
        private static string LoadTokenFromPrefs()
        {
            MainThreadGuard.AssertMainThread();

            // Unity Editor only: Get token from EditorUserSettings exclusively
            // Environment variables and EditorPrefs are explicitly ignored per A2-1 requirements
            var token = UnityEditor.EditorUserSettings.GetConfigValue("MCP.IpcToken");

            if (string.IsNullOrEmpty(token))
            {
                // Explicitly return null for missing/empty tokens (no development mode fallback)
                return null;
            }

            return token;
        }

        /// <summary>
        /// Load IPC port from Unity Editor settings
        /// </summary>
        private static int LoadPortFromPrefs()
        {
            MainThreadGuard.AssertMainThread();

            // Unity Editor only: Get port from EditorUserSettings
            var portString = UnityEditor.EditorUserSettings.GetConfigValue("MCP.IpcPort");

            if (string.IsNullOrEmpty(portString))
            {
                // Default to port 7777 if not configured
                return 7777;
            }

            if (int.TryParse(portString, out int port) && port > 0 && port <= 65535)
            {
                return port;
            }

            Debug.LogWarning($"[EditorIpcServer] Invalid port configuration '{portString}', using default 7777");
            return 7777;
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
                _writeLocks.TryRemove(stream, out _);

                if (_clientsByStream.Remove(stream, out var client))
                {
                    try { client.Close(); client.Dispose(); } catch { }
                }

                Debug.Log($"[EditorIpcServer] Unregistered stream, active count: {_activeStreams.Count}");
            }
        }
    }
}
