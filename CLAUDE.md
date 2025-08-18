# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Architecture

Unity MCP Server combines a Rust MCP server with Unity Editor bridge components in a bidirectional system. Currently in skeleton stage with minimal code but clear architectural direction.

**Core Architecture:**
- `server/` - Rust MCP server with multi-transport support (stdio/WebSocket) using rmcp SDK
- `bridge/` - Unity Editor tools for launching and coordinating with Rust server  
- Single repository approach for fast feedback loops, designed for future workspace expansion
- Communication between Rust MCP server and Unity Editor bridge uses **gRPC**

**gRPC Code Generation:**
- Protocol Buffers definitions in `proto/mcp/unity/v1/` (6 proto files: common, editor_control, assets, build, operations, events)
- Build script `server/build.rs` uses `tonic-prost-build` to generate Rust gRPC client/server code
- `server-stubs` feature flag controls server stub generation (essential for tests)
- Generated code placed in `server/src/generated/` module

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

# Tests (server-stubs feature required for integration tests)
cargo test --features server-stubs -- --nocapture
cargo test <module_or_test_name> --features server-stubs
cargo test # Unit tests only (no gRPC server stubs)

# Development workflow after proto changes
cargo clean  # Force rebuild when proto files change
cargo build --features server-stubs

# Run server locally
cargo run
```

**Unity Bridge (bridge/):**
```bash
# Tests (to be run via CI)
Unity -quit -batchmode -projectPath bridge -runTests -testResults results.xml -testPlatform EditMode
```

**Key Development Notes:**
- The `server-stubs` feature is essential for running integration tests that need gRPC server implementations
- Protocol buffer changes require clean rebuild to regenerate code properly
- CI/CD requires protoc 3.21.12 and runs matrix testing on Ubuntu/macOS
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
- `server/src/grpc/` - gRPC client components (config.rs for configuration, channel.rs for ChannelManager)
- `server/tests/` - Integration tests including smoke.rs for gRPC roundtrip health checks
- `proto/mcp/unity/v1/` - Protocol buffer definitions for all gRPC services
- `bridge/Assets/MCP/Editor/` - Unity Editor integration (MVP focus)
- `bridge/Packages/com.example.mcp-bridge/` - UPM package for reusability
- `docs/` - Architecture documentation

**Core Components:**
- `McpService` - Main MCP server implementation with unity_health tool for gRPC bridge communication
- `ChannelManager` - Manages gRPC connections with token-based authentication
- `BridgeConfig` / `ServerConfig` / `GrpcConfig` - Configuration management from environment variables
- Generated gRPC clients/servers from protocol buffers (in `src/generated/`)

## Testing Strategy

- Rust: Unit tests placed within modules using cfg(test)
- Integration tests in server/tests/ (require `--features server-stubs`)
- Tests should be deterministic and avoid network dependencies by default
- Unity: EditMode tests via Unity Test Runner
- `smoke.rs` provides gRPC roundtrip health check integration tests

## Commit Conventions

Use Conventional Commits format with English messages.

## Language Preferences

Responses should be provided in Japanese when interacting with this codebase.
