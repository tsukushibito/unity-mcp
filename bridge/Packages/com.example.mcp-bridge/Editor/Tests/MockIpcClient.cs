// Unity MCP Bridge - Mock IPC Client for Testing
// Simulates IPC communication without requiring actual Rust server
using System;
using System.Collections.Generic;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Threading;
using System.Threading.Tasks;
using UnityEngine;
using Google.Protobuf;
using Pb = Mcp.Unity.V1;

namespace Mcp.Unity.V1.Ipc.Tests
{
    /// <summary>
    /// Mock IPC client that simulates the Rust server behavior
    /// Used for testing cross-thread Unity API access patterns
    /// </summary>
    internal class MockIpcClient : IDisposable
    {
        private readonly IPAddress _address;
        private readonly int _port;
        private TcpClient _tcpClient;
        private NetworkStream _stream;
        private bool _disposed;
        private Pb.IpcWelcome _lastWelcome;

        public MockIpcClient(IPAddress address, int port)
        {
            _address = address ?? IPAddress.Loopback;
            _port = port;
        }

        /// <summary>
        /// Connect to the IPC server and perform handshake
        /// </summary>
        /// <summary>
        /// Connect to the IPC server and perform handshake
        /// </summary>
        public async Task<bool> ConnectAsync(string token = null, CancellationToken cancellationToken = default)
        {
            try
            {
                _tcpClient = new TcpClient();
                await _tcpClient.ConnectAsync(_address, _port);
                _stream = _tcpClient.GetStream();

                // Send T01 handshake
                var hello = new Pb.IpcHello
                {
                    IpcVersion = "1.0",
                    ClientName = "mock-test-client",
                    Token = token ?? "test-token",
                    ProjectRoot = Directory.GetCurrentDirectory(),
                    SchemaHash = Google.Protobuf.ByteString.CopyFromUtf8("mock-hash")
                };
                hello.Features.AddRange(new[] { 
                    "assets.basic",
                    "build.min",
                    "events.log"
                });

                var control = new Pb.IpcControl { Hello = hello };
                await SendControlFrameAsync(control);

                // Wait for welcome or reject
                var responseFrame = await Framing.ReadFrameAsync(_stream);
                if (responseFrame == null) return false;

                var responseControl = Pb.IpcControl.Parser.ParseFrom(responseFrame);
                
                if (responseControl.Welcome != null)
                {
                    Debug.Log($"[MockIpcClient] Handshake successful: {responseControl.Welcome.SessionId}");
                    _lastWelcome = responseControl.Welcome; // Store for inspection
                    return true;
                }
                else if (responseControl.Reject != null)
                {
                    Debug.LogWarning($"[MockIpcClient] Handshake rejected: {responseControl.Reject.Code} - {responseControl.Reject.Message}");
                    return false;
                }

                return false;
            }
            catch (Exception ex)
            {
                Debug.LogError($"[MockIpcClient] Connection failed: {ex.Message}");
                return false;
            }
        }

        /// <summary>
        /// Connect with specific version for version testing
        /// </summary>
        public async Task<bool> ConnectWithVersionAsync(string token, string version, CancellationToken cancellationToken = default)
        {
            try
            {
                _tcpClient = new TcpClient();
                await _tcpClient.ConnectAsync(_address, _port);
                _stream = _tcpClient.GetStream();

                // Send T01 handshake with specific version
                var hello = new Pb.IpcHello
                {
                    IpcVersion = version, // Use specified version
                    ClientName = "mock-test-client-version",
                    Token = token ?? "test-token",
                    ProjectRoot = Directory.GetCurrentDirectory(),
                    SchemaHash = Google.Protobuf.ByteString.CopyFromUtf8("mock-hash")
                };
                hello.Features.AddRange(new[] { 
                    "assets.basic",
                    "build.min",
                    "events.log"
                });

                var control = new Pb.IpcControl { Hello = hello };
                await SendControlFrameAsync(control);

                // Wait for welcome or reject
                var responseFrame = await Framing.ReadFrameAsync(_stream);
                if (responseFrame == null) return false;

                var responseControl = Pb.IpcControl.Parser.ParseFrom(responseFrame);
                
                if (responseControl.Welcome != null)
                {
                    Debug.Log($"[MockIpcClient] Version handshake successful: {responseControl.Welcome.SessionId}");
                    return true;
                }
                else if (responseControl.Reject != null)
                {
                    Debug.LogWarning($"[MockIpcClient] Version handshake rejected: {responseControl.Reject.Code} - {responseControl.Reject.Message}");
                    return false;
                }

                return false;
            }
            catch (Exception ex)
            {
                Debug.LogError($"[MockIpcClient] Version connection failed: {ex.Message}");
                return false;
            }
        }

        /// <summary>
        /// Connect with specific project root for path testing
        /// </summary>
        public async Task<bool> ConnectWithProjectRootAsync(string token, string projectRoot, CancellationToken cancellationToken = default)
        {
            try
            {
                _tcpClient = new TcpClient();
                await _tcpClient.ConnectAsync(_address, _port);
                _stream = _tcpClient.GetStream();

                // Send T01 handshake with specific project root
                var hello = new Pb.IpcHello
                {
                    IpcVersion = "1.0",
                    ClientName = "mock-test-client-path",
                    Token = token ?? "test-token",
                    ProjectRoot = projectRoot, // Use specified project root
                    SchemaHash = Google.Protobuf.ByteString.CopyFromUtf8("mock-hash")
                };
                hello.Features.AddRange(new[] { 
                    "assets.basic",
                    "build.min",
                    "events.log"
                });

                var control = new Pb.IpcControl { Hello = hello };
                await SendControlFrameAsync(control);

                // Wait for welcome or reject
                var responseFrame = await Framing.ReadFrameAsync(_stream);
                if (responseFrame == null) return false;

                var responseControl = Pb.IpcControl.Parser.ParseFrom(responseFrame);
                
                if (responseControl.Welcome != null)
                {
                    Debug.Log($"[MockIpcClient] Path handshake successful: {responseControl.Welcome.SessionId}");
                    _lastWelcome = responseControl.Welcome; // Store for inspection
                    return true;
                }
                else if (responseControl.Reject != null)
                {
                    Debug.LogWarning($"[MockIpcClient] Path handshake rejected: {responseControl.Reject.Code} - {responseControl.Reject.Message}");
                    return false;
                }

                return false;
            }
            catch (Exception ex)
            {
                Debug.LogError($"[MockIpcClient] Path connection failed: {ex.Message}");
                return false;
            }
        }

        /// <summary>
        /// Send Health request and return response
        /// </summary>
        public async Task<Pb.HealthResponse> SendHealthRequestAsync()
        {
            var correlationId = Guid.NewGuid().ToString();
            var request = new Pb.IpcRequest
            {
                Health = new Pb.HealthRequest()
            };

            var envelope = EnvelopeCodec.CreateRequest(correlationId, request);
            var bytes = EnvelopeCodec.Encode(envelope);
            await Framing.WriteFrameAsync(_stream, bytes);

            // Wait for response (skip events)
            while (true)
            {
                var responseFrame = await Framing.ReadFrameAsync(_stream);
                if (responseFrame == null)
                    throw new InvalidOperationException("No response received");

                var responseEnvelope = EnvelopeCodec.Decode(responseFrame);
                if (responseEnvelope.Response?.Health != null &&
                    responseEnvelope.CorrelationId == correlationId)
                {
                    return responseEnvelope.Response.Health;
                }

                // Ignore non-matching frames (events, other requests)
            }
        }

        /// <summary>
        /// Send Assets request from background thread (to trigger cross-thread issue)
        /// </summary>
        public async Task<Pb.AssetsResponse> SendAssetsRequestFromBackgroundThreadAsync()
        {
            return await Task.Run(async () =>
            {
#if UNITY_EDITOR && DEBUG
                Mcp.Unity.V1.Ipc.Infra.Diag.Log("Sending Assets request from background thread");
#endif
                
                var correlationId = Guid.NewGuid().ToString();
                var request = new Pb.IpcRequest
                {
                    Assets = new Pb.AssetsRequest
                    {
                        Refresh = new Pb.RefreshRequest { Force = false }
                    }
                };

                var envelope = EnvelopeCodec.CreateRequest(correlationId, request);
                var bytes = EnvelopeCodec.Encode(envelope);
                await Framing.WriteFrameAsync(_stream, bytes);

                // Wait for response
                var responseFrame = await Framing.ReadFrameAsync(_stream);
                if (responseFrame == null)
                {
                    return new Pb.AssetsResponse { StatusCode = 13, Message = "no response received" };
                }

                var responseEnvelope = EnvelopeCodec.Decode(responseFrame);
                if (responseEnvelope.Response?.Assets == null)
                {
                    return new Pb.AssetsResponse { StatusCode = 13, Message = "invalid assets response" };
                }

                return responseEnvelope.Response.Assets;
            });
        }

        /// <summary>
        /// Send Build request from background thread (to trigger cross-thread issue)
        /// </summary>
        public async Task<Pb.BuildResponse> SendBuildRequestFromBackgroundThreadAsync()
        {
            return await Task.Run(async () =>
            {
#if UNITY_EDITOR && DEBUG
                Mcp.Unity.V1.Ipc.Infra.Diag.Log("Sending Build request from background thread");
#endif

                var correlationId = Guid.NewGuid().ToString();
                var request = new Pb.IpcRequest
                {
                    Build = new Pb.BuildRequest
                    {
                        Bundles = new Pb.BuildAssetBundlesRequest
                        {
                            OutputDirectory = "Temp/TestBundles",
                            ForceRebuild = false,
                            ChunkBased = false
                        }
                    }
                };

                var envelope = EnvelopeCodec.CreateRequest(correlationId, request);
                var bytes = EnvelopeCodec.Encode(envelope);
                await Framing.WriteFrameAsync(_stream, bytes);

                // Wait for response
                var responseFrame = await Framing.ReadFrameAsync(_stream);
                if (responseFrame == null)
                {
                    return new Pb.BuildResponse { Bundles = new Pb.BuildAssetBundlesResponse { StatusCode = 13, Message = "no response received" } };
                }

                var responseEnvelope = EnvelopeCodec.Decode(responseFrame);
                if (responseEnvelope.Response?.Build == null)
                {
                    return new Pb.BuildResponse { Bundles = new Pb.BuildAssetBundlesResponse { StatusCode = 13, Message = "invalid build response" } };
                }

                return responseEnvelope.Response.Build;
            });
        }

        /// <summary>
        /// Simulate validation calls from background thread
        /// </summary>
        public async Task SimulateEditorStateValidationFromBackgroundThread()
        {
            await Task.Run(() =>
            {
#if UNITY_EDITOR && DEBUG
                Mcp.Unity.V1.Ipc.Infra.Diag.Log("Simulating editor state validation from background thread");
                
                // These calls would normally be in ValidateEditorState() method
                // but we're calling them from background thread to trigger cross-thread violations
                try
                {
                    var isCompiling = UnityEditor.EditorApplication.isCompiling;
                    var isUpdating = UnityEditor.EditorApplication.isUpdating;
                    var version = UnityEngine.Application.unityVersion;
                    
                    Debug.Log($"Direct Unity API access from BG thread: isCompiling={isCompiling}, isUpdating={isUpdating}, version={version}");
                }
                catch (Exception ex)
                {
                    Debug.LogWarning($"Cross-thread Unity API access failed: {ex.Message}");
                }
#endif
            });
        }

        /// <summary>
        /// Get the last received welcome message for testing
        /// </summary>
        public Pb.IpcWelcome GetLastWelcomeInfo()
        {
            return _lastWelcome;
        }

        private async Task SendControlFrameAsync(Pb.IpcControl control)
        {
            var bytes = control.ToByteArray();
            await Framing.WriteFrameAsync(_stream, bytes);
        }

        public void Dispose()
        {
            if (_disposed) return;

            try
            {
                _stream?.Close();
                _tcpClient?.Close();
            }
            catch (Exception ex)
            {
                Debug.LogWarning($"[MockIpcClient] Dispose error: {ex.Message}");
            }
            finally
            {
                _stream = null;
                _tcpClient = null;
                _disposed = true;
            }
        }

        /// <summary>
        /// Try to find an available port for testing
        /// </summary>
        public static int FindAvailablePort(int startPort = 8000)
        {
            for (int port = startPort; port < startPort + 100; port++)
            {
                try
                {
                    var listener = new TcpListener(IPAddress.Loopback, port);
                    listener.Start();
                    listener.Stop();
                    return port;
                }
                catch
                {
                    // Port is in use, try next one
                }
            }
            throw new InvalidOperationException("No available ports found");
        }
    }
}
