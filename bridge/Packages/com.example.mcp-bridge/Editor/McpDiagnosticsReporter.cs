using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.Compilation;
using UnityEngine;
using System.Security.Cryptography;

namespace MCP.Editor
{
    [Serializable]
    public class DiagnosticRange
    {
        public DiagnosticPosition start;
        public DiagnosticPosition end;

        public DiagnosticRange(int line, int character)
        {
            start = new DiagnosticPosition { line = line, character = character };
            end = new DiagnosticPosition { line = line, character = character };
        }
    }

    [Serializable]
    public class DiagnosticPosition
    {
        public int line;
        public int character;
    }

    [Serializable]
    public class CompilerMessageWithAssembly
    {
        public CompilerMessage message;
        public string assemblyName;

        public CompilerMessageWithAssembly(CompilerMessage message, string assemblyName)
        {
            this.message = message;
            this.assemblyName = assemblyName;
        }
    }

    [Serializable]
    public class Diagnostic
    {
        public string file_uri;
        public DiagnosticRange range;
        public string severity;
        public string message;
        public string code;
        public string assembly;
        public string source;
        public string fingerprint;
        public string first_seen;
        public string last_seen;
    }

    [Serializable]
    public class DiagnosticSummary
    {
        public int errors;
        public int warnings;
        public int infos;
        public string[] assemblies;
    }

    [Serializable]
    public class CompileDiagnostics
    {
        public string compile_id;
        public DiagnosticSummary summary;
        public Diagnostic[] diagnostics;
        public bool truncated;
    }

    [InitializeOnLoad]
    public static class McpDiagnosticsReporter
    {
        private static readonly string ProjectRoot;
        private static readonly string LatestJsonPath;
        private static readonly string OutputDirectory;
        private static readonly List<CompilerMessageWithAssembly> CollectedMessages = new List<CompilerMessageWithAssembly>();

        static McpDiagnosticsReporter()
        {
            ProjectRoot = Path.GetFullPath(Path.Combine(Application.dataPath, ".."));

            var envPath = Environment.GetEnvironmentVariable("UNITY_MCP_DIAG_PATH");
            if (!string.IsNullOrEmpty(envPath))
            {
                LatestJsonPath = Path.IsPathRooted(envPath)
                    ? envPath
                    : Path.Combine(ProjectRoot, envPath);
            }
            else
            {
                LatestJsonPath = Path.Combine(ProjectRoot, "Temp/AI/latest.json");
            }

            OutputDirectory = Path.GetDirectoryName(LatestJsonPath);

            CompilationPipeline.compilationStarted += OnCompilationStarted;
            CompilationPipeline.compilationFinished += OnCompilationFinished;
            CompilationPipeline.assemblyCompilationFinished += OnAssemblyCompilationFinished;

            // Ensure output directory exists
            EnsureOutputDirectory();
        }

        private static void EnsureOutputDirectory()
        {
            try
            {
                if (!Directory.Exists(OutputDirectory))
                {
                    Directory.CreateDirectory(OutputDirectory);
                }
            }
            catch (Exception e)
            {
                Debug.LogError($"[McpDiagnosticsReporter] Failed to create output directory: {e.Message}");
            }
        }

        private static void OnCompilationStarted(object obj)
        {
            // Clear previous diagnostics when compilation starts
            try
            {
                // Ensure no residue from previous compilations
                CollectedMessages.Clear();
                if (File.Exists(LatestJsonPath))
                {
                    // Keep backup for comparison if needed
                    var backupPath = Path.Combine(OutputDirectory, "previous.json");
                    File.Copy(LatestJsonPath, backupPath, true);
                }
            }
            catch (Exception e)
            {
                Debug.LogError($"[McpDiagnosticsReporter] Failed to backup previous diagnostics: {e.Message}");
            }
        }

        private static void OnCompilationFinished(object obj)
        {
            // Compilation finished, collect and output diagnostics
            CollectAndOutputDiagnostics();
        }

        private static void OnAssemblyCompilationFinished(string assemblyName, CompilerMessage[] messages)
        {
            // 集約: アセンブリごとのメッセージを貯めて、終了時に一括でJSON化
            if (messages == null || messages.Length == 0) return;
            CollectedMessages.AddRange(messages.Select(m => 
                new CompilerMessageWithAssembly(m, assemblyName)
            ));
        }

        private static void CollectAndOutputDiagnostics()
        {
            try
            {
                var compileId = DateTimeOffset.UtcNow.ToUnixTimeSeconds().ToString();
                var diagnostics = new List<Diagnostic>();
                var assemblyNames = new HashSet<string>();

                foreach (var msgWithAssembly in CollectedMessages)
                {
                    if (string.IsNullOrEmpty(msgWithAssembly.message.file)) continue;

                    var diagnostic = CreateDiagnostic(msgWithAssembly.message, msgWithAssembly.assemblyName);
                    if (diagnostic != null)
                    {
                        diagnostics.Add(diagnostic);
                        if (!string.IsNullOrEmpty(diagnostic.assembly))
                            assemblyNames.Add(diagnostic.assembly);
                    }
                }

                // 次回に備えてクリア
                CollectedMessages.Clear();

                // Create summary
                var summary = new DiagnosticSummary
                {
                    errors = diagnostics.Count(d => d.severity == "error"),
                    warnings = diagnostics.Count(d => d.severity == "warning"),
                    infos = diagnostics.Count(d => d.severity == "info"),
                    assemblies = assemblyNames.ToArray()
                };

                var compileDiagnostics = new CompileDiagnostics
                {
                    compile_id = compileId,
                    summary = summary,
                    diagnostics = diagnostics.ToArray(),
                    truncated = false // MVP: no truncation yet
                };

                // Output to JSON
                OutputDiagnostics(compileDiagnostics, compileId);
            }
            catch (Exception e)
            {
                Debug.LogError($"[McpDiagnosticsReporter] Failed to collect diagnostics: {e.Message}");
            }
        }

        private static Diagnostic CreateDiagnostic(CompilerMessage msg, string assemblyName)
        {
            try
            {
                // Convert file path to URI
                var filePath = Path.GetFullPath(msg.file);
                var fileUri = new Uri(filePath).AbsoluteUri;

                // Determine severity
                var severity = msg.type switch
                {
                    CompilerMessageType.Error => "error",
                    CompilerMessageType.Warning => "warning",
                    _ => "info"
                };

                // Use the actual assembly name passed from assemblyCompilationFinished
                var assembly = assemblyName;

                // Create range (line is 1-based in Unity, 0-based in LSP)
                var line = Math.Max(0, msg.line - 1);
                var range = new DiagnosticRange(line, Math.Max(0, msg.column));

                // Generate fingerprint
                var fingerprint = GenerateFingerprint(filePath, msg.line, msg.message, assembly);

                var diagnostic = new Diagnostic
                {
                    file_uri = fileUri,
                    range = range,
                    severity = severity,
                    message = msg.message,
                    code = ExtractErrorCode(msg.message),
                    assembly = assembly,
                    source = "Unity",
                    fingerprint = fingerprint,
                    first_seen = DateTime.UtcNow.ToString("yyyy-MM-ddTHH:mm:ss.fffZ"),
                    last_seen = DateTime.UtcNow.ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
                };

                return diagnostic;
            }
            catch (Exception e)
            {
                Debug.LogError($"[McpDiagnosticsReporter] Failed to create diagnostic: {e.Message}");
                return null;
            }
        }

        private static string ExtractErrorCode(string message)
        {
            // Extract error codes like CS0103, CS1061, etc.
            var match = System.Text.RegularExpressions.Regex.Match(message, @"\b(CS\d{4})\b");
            return match.Success ? match.Value : null;
        }

        private static string GenerateFingerprint(string filePath, int line, string message, string assembly)
        {
            var input = $"{filePath}|{line}|{message}|{assembly}";
            using (var sha = SHA256.Create())
            {
                var bytes = System.Text.Encoding.UTF8.GetBytes(input);
                var hash = sha.ComputeHash(bytes);
                return BitConverter.ToString(hash).Replace("-", string.Empty).ToLowerInvariant();
            }
        }

        private static void OutputDiagnostics(CompileDiagnostics diagnostics, string compileId)
        {
            try
            {
                EnsureOutputDirectory();

                var json = JsonUtility.ToJson(diagnostics, prettyPrint: true);

                // Write latest.json
                File.WriteAllText(LatestJsonPath, json);

                // Write compile-<id>.json
                var idSpecificPath = Path.Combine(OutputDirectory, $"compile-{compileId}.json");
                File.WriteAllText(idSpecificPath, json);

                Debug.Log($"[McpDiagnosticsReporter] Diagnostics written to {LatestJsonPath} " +
                         $"({diagnostics.summary.errors} errors, {diagnostics.summary.warnings} warnings, " +
                         $"{diagnostics.summary.infos} infos)");
            }
            catch (Exception e)
            {
                Debug.LogError($"[McpDiagnosticsReporter] Failed to write diagnostics: {e.Message}");
            }
        }
    }
}
