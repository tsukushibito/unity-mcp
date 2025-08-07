# CRUSH.md

Repo: Rust MCP server (Tokio, rmcp, axum). No existing tests found; use Cargo defaults.

Build/run
- Build: cargo build
- Release build: cargo build --release
- Run: cargo run
- Env logging: RUST_LOG=debug cargo run

Test
- Run all: cargo test
- Single test by name: cargo test <name>
- Single test in file: cargo test --test <file_stem> -- --ignored
- Show output: cargo test -- --nocapture

Lint/format
- Format: cargo fmt --all
- Lint: cargo clippy --all-targets --all-features -- -D warnings

Code style
- Edition 2021; use explicit imports at module top, group std, external, crate, self; alphabetize within groups.
- Formatting via rustfmt defaults; 100 cols; no trailing commas changes beyond rustfmt.
- Types: prefer Result<T, anyhow::Error>; bubble errors with ?; avoid unwrap/expect in non-tests.
- Naming: snake_case for funcs/vars/modules; CamelCase for types/traits; SCREAMING_SNAKE_CASE for consts.
- Errors/logging: use tracing; levels: error for failures, warn for recoverable, info for milestones, debug for flow. Do not log secrets.
- Async: use tokio; avoid blocking; spawn tasks with tokio::spawn; prefer structured concurrency.
- Serialization: derive serde on data structs; validate external inputs.
- API: keep handlers thin; move logic to modules; keep Unity tool handlers small and testable.

Project notes
- Entry: src/main.rs; server/router: src/unity.rs using rmcp macros.
- Config: config/default.toml (if used by future features).
- Add tests in tests/ or src with #[cfg(test)]. Use tokio::test for async.

AI assistant rules
- If .cursor/rules or .cursorrules or .github/copilot-instructions.md appear later, mirror their guidance here.
