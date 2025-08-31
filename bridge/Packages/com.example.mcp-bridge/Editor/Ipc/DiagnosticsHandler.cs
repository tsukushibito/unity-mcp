// Unity MCP Bridge - Diagnostics Handler
// Handles compile diagnostics requests via IPC
using System;
using UnityEngine;
using MCP.Editor;
using Pb = Mcp.Unity.V1;

namespace Mcp.Unity.V1.Ipc
{
    internal static class DiagnosticsHandler
    {
        /// <summary>
        /// Handle GetCompileDiagnostics request
        /// </summary>
        /// <param name="request">The diagnostics request</param>
        /// <returns>Diagnostics response</returns>
        public static Pb.GetCompileDiagnosticsResponse Handle(Pb.GetCompileDiagnosticsRequest request)
        {
            Debug.Log($"[DiagnosticsHandler] Processing diagnostics request: maxItems={request.MaxItems}, severity={request.Severity}, assembly={request.Assembly}");

            try
            {
                // Get diagnostics from memory cache
                var diagnostics = McpDiagnosticsReporter.GetCachedDiagnostics(
                    request.MaxItems > 0 ? request.MaxItems : 2000,
                    string.IsNullOrEmpty(request.Severity) ? "all" : request.Severity,
                    request.ChangedOnly,
                    string.IsNullOrEmpty(request.Assembly) ? null : request.Assembly
                );

                if (diagnostics == null)
                {
                    // No diagnostics available
                    return new Pb.GetCompileDiagnosticsResponse
                    {
                        Success = false,
                        ErrorMessage = "No compile diagnostics available. Please trigger a compilation in Unity Editor first."
                    };
                }

                // Convert from Unity format to protobuf format
                var pbDiagnostics = new Pb.CompileDiagnostic[diagnostics.diagnostics.Length];
                for (int i = 0; i < diagnostics.diagnostics.Length; i++)
                {
                    var diag = diagnostics.diagnostics[i];
                    pbDiagnostics[i] = new Pb.CompileDiagnostic
                    {
                        FileUri = diag.file_uri ?? "",
                        Range = new Pb.DiagnosticRange
                        {
                            Line = (uint)diag.range.line,
                            Column = (uint)diag.range.column
                        },
                        Severity = diag.severity ?? "",
                        Message = diag.message ?? "",
                        Code = diag.code ?? "",
                        Assembly = diag.assembly ?? "",
                        Source = diag.source ?? "",
                        Fingerprint = diag.fingerprint ?? "",
                        FirstSeen = diag.first_seen ?? "",
                        LastSeen = diag.last_seen ?? ""
                    };
                }

                var pbSummary = new Pb.DiagnosticSummary
                {
                    Errors = (uint)diagnostics.summary.errors,
                    Warnings = (uint)diagnostics.summary.warnings,
                    Infos = (uint)diagnostics.summary.infos
                };
                pbSummary.Assemblies.AddRange(diagnostics.summary.assemblies ?? new string[0]);

                var response = new Pb.GetCompileDiagnosticsResponse
                {
                    Success = true,
                    ErrorMessage = "",
                    CompileId = diagnostics.compile_id ?? "",
                    Summary = pbSummary,
                    Truncated = diagnostics.truncated
                };
                response.Diagnostics.AddRange(pbDiagnostics);

                Debug.Log($"[DiagnosticsHandler] Returning {pbDiagnostics.Length} diagnostics (errors={pbSummary.Errors}, warnings={pbSummary.Warnings})");
                return response;
            }
            catch (Exception ex)
            {
                Debug.LogError($"[DiagnosticsHandler] Failed to handle diagnostics request: {ex.Message}");
                return new Pb.GetCompileDiagnosticsResponse
                {
                    Success = false,
                    ErrorMessage = $"Internal error: {ex.Message}"
                };
            }
        }
    }
}