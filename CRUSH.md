# CRUSH.md

Project type: Unity MCP server skeleton (Rust server + Unity bridge planned). Currently minimal; follow these conventions until code lands.

Build/lint/test
- Rust (server/): cargo build; cargo check; cargo fmt --all -- --check; cargo clippy --all-targets -- -D warnings; cargo test
- Run a single Rust test: cargo test <module_or_test_name>
- Unity (bridge/ via CI later): Use Unity -quit -batchmode -projectPath bridge -runTests -testResults results.xml -testPlatform EditMode
- Scripts: prefer scripts/ wrappers if added later; Dev Container sets up toolchains.

Code style
- Language: Rust for server (rmcp, tracing), C# for Unity Editor/Runtime.
- Imports: in Rust group std, external crates, local mods; in C# use System first, then UnityEngine/UnityEditor, then project namespaces.
- Formatting: rustfmt default; C# via EditorConfig/Unity defaults. No trailing spaces; 100–120 col soft limit.
- Types: prefer explicit types at public boundaries; avoid unwrap/expect in prod paths; use Result<T, E> and thiserror anyhow as appropriate.
- Naming: snake_case for Rust items, CamelCase for types/structs/enums, SCREAMING_SNAKE_CASE for consts; C# uses PascalCase types/methods, camelCase fields, UPPER_CASE consts.
- Error handling: use anyhow for app-level errors and thiserror for domain errors; map errors with context using anyhow::Context; never panic in request handlers; in C# use try/catch and log via UnityEngine.Debug.
- Logging: tracing with levels (error,warn,info,debug,trace); no secrets in logs.
- Concurrency: prefer async (tokio) when introduced; avoid blocking in async contexts.
- Configuration: TOML in server/config; support override via CLI flags later.
- Testing: unit tests colocated in Rust modules (cfg(test)); integration tests in server/tests when added. Keep tests deterministic; no network by default.
- Git: small focused commits; no generated files; add .crush cache dir to .gitignore.

Tools and rules
- If .cursor/rules or .cursorrules or .github/copilot-instructions.md appear, incorporate them here and follow highest‑specificity local rules.
- Keep this file ~20–30 lines; update as server/ and bridge/ directories are added.
