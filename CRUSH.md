# CRUSH.md

Project type: Unity MCP server skeleton (Rust server + Unity bridge). Keep this concise guide updated as code evolves.

Build/lint/test
- Rust (server/): cargo build; cargo check; cargo fmt --all -- --check; cargo clippy --all-targets -- -D warnings; cargo test
- Run a single Rust test: cargo test <module_or_test_name> or cargo test <module>::<test_name>
- Run with logs: RUST_LOG=debug cargo run -p server
- Unity (bridge/, via CI later): Unity -quit -batchmode -projectPath bridge -runTests -testResults results.xml -testPlatform EditMode
- Prefer scripts/ wrappers if added; Dev Container provides toolchains

Code style
- Languages: Rust (server with rmcp, tokio, tracing), C# (Unity Editor/Runtime)
- Imports: Rust group std, external crates, local mods; C# order System, UnityEngine/UnityEditor, then project namespaces
- Formatting: rustfmt default; cargo fmt must pass; C# via Unity/EditorConfig defaults; 100–120 col soft limit, no trailing spaces
- Types: explicit types at public boundaries; avoid unwrap/expect in prod paths; use Result<T, E>; anyhow for app-level, thiserror for domain
- Naming: Rust snake_case items, CamelCase types/enums/structs, SCREAMING_SNAKE_CASE consts; C# PascalCase types/methods, camelCase fields, UPPER_CASE consts
- Errors: add context via anyhow::Context; never panic in handlers; C# try/catch with UnityEngine.Debug logging
- Logging: tracing with levels (error,warn,info,debug,trace); no secrets or PII in logs; EnvFilter via RUST_LOG
- Concurrency: async-first (tokio); avoid blocking in async; spawn tasks sparingly; propagate cancellation
- Configuration: TOML in server/config; allow env overrides via RUST_*; CLI flags may override later
- Testing: Rust unit tests with cfg(test) colocated; future integration tests in server/tests; deterministic, no network by default
- Git hygiene: small focused commits, no generated files

Editor/AI rules
- If .cursor/rules or .cursorrules or .github/copilot-instructions.md appear, follow them with highest specificity and mirror in this file
- Keep this file ~20–30 lines; expand as server/ and bridge/ mature