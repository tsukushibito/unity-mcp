# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Architecture

Unity MCP Server combines a Rust MCP server with Unity Editor bridge components in a bidirectional system. Currently in skeleton stage with minimal code but clear architectural direction.

**Core Architecture:**
- `server/` - Rust MCP server with multi-transport support (stdio/WebSocket) using rmcp SDK
- `bridge/` - Unity Editor tools for launching and coordinating with Rust server  
- Single repository approach for fast feedback loops, designed for future workspace expansion
- Communication between Rust MCP server and Unity Editor bridge uses **Direct IPC**

**IPC Protocol Generation:**
- Protocol Buffers definitions in `proto/mcp/unity/v1/` (7 proto files: common, editor_control, assets, build, operations, events, ipc)
- Build script `server/build.rs` uses `prost-build` to generate Rust protocol buffer message types
- Generated code placed in `server/src/generated/` module
- IPC uses length-delimited framing over Unix domain sockets (Linux/macOS) or Named pipes (Windows)

## Development Environment

This project is designed to work in VS Code workspaces or similar development environments. The current working directory should be `/workspaces/unity-mcp/server` for Rust development.

## Development Commands

**Important: All Rust commands must be run from the `server/` directory.**

**Rust Server (server/):**
```bash
# Build and check
cargo build --locked
cargo check
cargo fmt --check  # Check formatting
cargo fmt           # Apply formatting
cargo clippy --all-targets -- -D warnings

# Tests
cargo test -- --nocapture
cargo test <module_or_test_name>
cargo test # All tests including IPC integration tests

# Protocol Buffer generation (when proto files change)
./scripts/generate-rust-proto.sh  # Generate Rust structs from proto files
git add src/generated/             # Commit generated files
git commit -m "regenerate proto files"

# Development workflow
cargo build  # Fast build (no proto generation)

# Run server locally
cargo run
```

**Unity Bridge (bridge/):**
```bash
# Tests (to be run via CI)
Unity -quit -batchmode -projectPath bridge -runTests -testResults results.xml -testPlatform EditMode
```

**Key Development Notes:**
- Protocol buffer changes require manual regeneration using `./scripts/generate-rust-proto.sh`
- Generated proto files are committed to Git for consistent builds
- CI/CD requires protoc 3.21.12 and runs matrix testing on Ubuntu/macOS
- IPC endpoints are OS-specific: Unix domain sockets on Unix-like systems, Named pipes on Windows
- Use scripts/ directory for additional wrapper commands

## Code Conventions

**Languages and Frameworks:**
- Rust for server (rmcp, tracing, tokio for async)
- C# for Unity Editor/Runtime components

**Import Organization:**
- Rust: std → external crates → local modules
- C#: System → UnityEngine/UnityEditor → project namespaces

**Naming Conventions:**
- Rust: snake_case items, CamelCase types, SCREAMING_SNAKE_CASE constants
- C#: PascalCase types/methods, camelCase fields, UPPER_CASE constants

**Error Handling:**
- Rust: anyhow for application-level, thiserror for domain errors, avoid unwrap/expect in production
- C#: try/catch with UnityEngine.Debug logging
- No panics in request handlers

**Module Structure:**
- Use `module_name.rs` instead of `mod.rs` for submodules (Rust 2018+ convention)
- Avoid `mod.rs` files except for the root module

**Configuration:**
- TOML files in server/config/
- CLI flag overrides planned for future

## Project Structure

**Key Directories:**
- `server/src/ipc/` - IPC client components (client.rs for IpcClient, codec.rs for protocol encoding)
- `server/tests/` - Integration tests including ipc_integration.rs for IPC handshake and health checks
- `proto/mcp/unity/v1/` - Protocol buffer definitions for all IPC services
- `bridge/Assets/MCP/Editor/` - Unity Editor integration (MVP focus)
- `bridge/Packages/com.example.mcp-bridge/` - UPM package for reusability
- `docs/` - Architecture documentation

**Core Components:**
- `McpService` - Main MCP server implementation with unity_health tool for IPC bridge communication
- `IpcClient` - Manages IPC connections with token-based authentication and correlation ID tracking
- `IpcConfig` - Configuration management from environment variables (MCP_IPC_ENDPOINT, MCP_IPC_TOKEN)
- Generated protocol buffer message types from proto definitions (in `src/generated/`)

## Testing Strategy

- Rust: Unit tests placed within modules using cfg(test)
- Integration tests in server/tests/ with TCP echo server for IPC validation
- Tests should be deterministic and avoid network dependencies by default
- Unity: EditMode tests via Unity Test Runner
- `ipc_integration.rs` provides IPC handshake and health check integration tests

## Commit Conventions

Use Conventional Commits format with English messages.

## Language Preferences

Responses should be provided in Japanese when interacting with this codebase.
