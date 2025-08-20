// Unity MCP Bridge - Path Policy
// Security policy for build output path validation
using System;
using System.IO;

namespace Mcp.Unity.V1.Ipc
{
    internal static class PathPolicy
    {
        static readonly string ProjectRoot = Directory.GetCurrentDirectory().Replace('\\','/');
        static readonly string BuildsRoot  = Path.Combine(ProjectRoot, "Builds").Replace('\\','/');
        static readonly string AbRoot      = Path.Combine(ProjectRoot, "AssetBundles").Replace('\\','/');

        /// <summary>
        /// Check if child path is under parent directory
        /// </summary>
        static bool IsUnder(string child, string parent)
            => child.StartsWith(parent.EndsWith("/") ? parent : parent + "/", StringComparison.OrdinalIgnoreCase)
            || string.Equals(child, parent, StringComparison.OrdinalIgnoreCase);

        /// <summary>
        /// Check if path is system directory (should be forbidden)
        /// </summary>
        static bool IsSystemPath(string full)
        {
#if UNITY_EDITOR_WIN
            full = full.ToLowerInvariant();
            if (full.StartsWith(@"c:\windows\")) return true;
            if (full.StartsWith(@"c:\program files\")) return true;
            if (full.StartsWith(@"\\")) return true; // UNC
            var root = Path.GetPathRoot(full);
            if (string.Equals(full.TrimEnd('\\','/'), root?.TrimEnd('\\','/'), StringComparison.OrdinalIgnoreCase)) return true;
            return false;
#else
            if (full == "/" || full.StartsWith("/usr/") || full.StartsWith("/bin/") || full.StartsWith("/etc/")) return true;
            return false;
#endif
        }

        /// <summary>
        /// Validate and resolve player build output path
        /// </summary>
        public static bool TryResolvePlayerOutput(string input, out string resolved, out string error)
        {
            resolved = error = null;
            if (string.IsNullOrWhiteSpace(input)) { error = "output_path required"; return false; }

            var full = Path.GetFullPath(Path.IsPathRooted(input) ? input : Path.Combine(ProjectRoot, input)).Replace('\\','/');
            if (IsSystemPath(full)) { error = "system path not allowed"; return false; }

            // Forbidden: Assets/ and Library/
            if (IsUnder(full, Path.Combine(ProjectRoot, "Assets").Replace('\\','/')) ||
                IsUnder(full, Path.Combine(ProjectRoot, "Library").Replace('\\','/')))
            { error = "output under Assets/Library is forbidden"; return false; }

            // Allowed: Builds/ or outside project root
            var allowed = IsUnder(full, BuildsRoot) || !IsUnder(full, ProjectRoot);
            if (!allowed) { error = $"must be under {BuildsRoot}/ or outside project root"; return false; }

            // Create parent directory
            var parent = Path.GetDirectoryName(full);
            if (string.IsNullOrEmpty(parent)) { error = "invalid output path"; return false; }
            Directory.CreateDirectory(parent);

            resolved = full;
            return true;
        }

        /// <summary>
        /// Validate and resolve asset bundles output directory
        /// </summary>
        public static bool TryResolveBundlesOutput(string input, out string resolved, out string error)
        {
            resolved = error = null;
            if (string.IsNullOrWhiteSpace(input)) { error = "output_directory required"; return false; }

            var full = Path.GetFullPath(Path.IsPathRooted(input) ? input : Path.Combine(ProjectRoot, input)).Replace('\\','/');
            if (IsSystemPath(full)) { error = "system path not allowed"; return false; }

            // Allowed: AssetBundles/ or Builds/AssetBundles/ or outside project root
            var buildsAb = Path.Combine(BuildsRoot, "AssetBundles").Replace('\\','/');
            var allowed = IsUnder(full, AbRoot) || IsUnder(full, buildsAb) || !IsUnder(full, ProjectRoot);
            if (!allowed) { error = $"must be under {AbRoot}/ or {buildsAb}/ or outside project root"; return false; }

            Directory.CreateDirectory(full);
            resolved = full;
            return true;
        }
    }
}