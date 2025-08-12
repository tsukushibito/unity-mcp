# Objective
Set up a **deterministic, reproducible** build that compiles `proto3` gRPC definitions into Rust client stubs using `tonic`/`prost` for the **Rust server-side only** (no googleapis, L0 policy). Output must build on macOS and Ubuntu CI without manual steps.

---

## Status & Change Policy
**Status:** PROVISIONAL (work-in-progress). These `.proto` files are intentionally minimal for L0 and **may change with breaking changes** while design is being finalized.

- **Package stays:** `mcp.unity.v1`.
- **Before *Schema Freeze***: breaking changes are allowed (field renumbering, message/service renames/splits/merges, streaming payload redesigns). Consumers must regenerate code on each change.
- **Schema Freeze criteria:** end-to-end flows (EditorControl, Assets, Build, Operations, Events) verified against the Bridge; freeze is recorded by tagging the repo `schema-freeze-v1` and adding `docs/SCHEMA_FREEZE.md` with the commit hash.
- **After freeze:** only backward-compatible additions (new optional fields with new numbers; no renumbering; use `reserved` for removals). For breaking needs, branch to `mcp.unity.v2`.

---

## Scope constraints
- **Language/stack**: Rust (stable), tonic/prost, protoc.
- **gRPC role in Rust**: **client stubs only** (`build_server(false)`), because the Unity-side Bridge will expose the gRPC server.
- **L0**: No `google.rpc.*` imports; use plain `proto3` messages.
- **Repo layout**: Single workspace with `server` crate and shared `proto/` directory.

---

## Tooling versions (pin for reproducibility)
- Rust toolchain: `stable` via `rustup` (e.g., 1.79+).
- `protoc` (Protocol Buffers): **≥ 3.21**.
- Crates:
  - `tonic = 0.11`
  - `tonic-build = 0.11`
  - `prost = 0.12`
  - `tokio = 1.*` with `macros`, `rt-multi-thread` features

> If your local versions differ, keep API compatibility with these major/minor lines. CI will enforce build.

---

## Directory layout (authoritative)
```
repo-root/
├─ proto/
│  └─ mcp/unity/v1/
│     ├─ common.proto
│     ├─ editor_control.proto
│     ├─ assets.proto
│     ├─ build.proto
│     ├─ operations.proto
│     └─ events.proto
└─ server/
   ├─ Cargo.toml
   ├─ build.rs
   └─ src/
      └─ main.rs
```

---

## Step 0 — Install prerequisites
**macOS**
```bash
brew install protobuf rustup-init
rustup toolchain install stable
rustup default stable
protoc --version  # must be >= 3.21
```
**Ubuntu**
```bash
sudo apt-get update
sudo apt-get install -y protobuf-compiler curl build-essential pkg-config
curl https://sh.rustup.rs -sSf | sh -s -- -y
source "$HOME/.cargo/env"
rustup default stable
protoc --version  # must be >= 3.21
```
**Acceptance:** `protoc --version` prints ≥ 3.21, `rustc --version` prints stable.

---

## Step 1 — Create repo skeleton
```bash
mkdir -p repo-root/proto/mcp/unity/v1
cd repo-root
cargo new server --bin
```
**Acceptance:** `repo-root/server/Cargo.toml` and `src/main.rs` exist.

---

## Step 2 — Add Rust dependencies (server/Cargo.toml)
Replace the contents of `server/Cargo.toml` with **exactly** the following:
```toml
[package]
name = "server"
version = "0.1.0"
edition = "2021"

[dependencies]
ton ic = { version = "0.11", features = ["transport", "tls"] }
prost = "0.12"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
anyhow = "1"

[build-dependencies]
tonic-build = "0.11"
```
**Acceptance:** `cargo metadata` succeeds.

---

## Step 3 — Author minimal L0 proto files (compilable now)
Create **all six** files exactly as specified. Do not change `package` names.

**Add this header comment at the top of every file:**
```proto
// STATUS: PROVISIONAL — Breaking changes allowed until Schema Freeze.
// PACKAGE: mcp.unity.v1
```


**proto/mcp/unity/v1/common.proto**
```proto
syntax = "proto3";
package mcp.unity.v1;

// Generic empty placeholder to avoid google.protobuf.Empty at L0.
message Empty {}

// Minimal operation reference for streaming/event demos.
message OperationRef {
  string id = 1; // opaque identifier
}
```

**proto/mcp/unity/v1/editor_control.proto**
```proto
syntax = "proto3";
package mcp.unity.v1;
import "mcp/unity/v1/common.proto";

service EditorControl {
  rpc Health(HealthRequest) returns (HealthResponse);
  rpc GetPlayMode(Empty) returns (GetPlayModeResponse);
  rpc SetPlayMode(SetPlayModeRequest) returns (SetPlayModeResponse);
}

message HealthRequest {}
message HealthResponse { string status = 1; /* e.g., "OK" */ }

message GetPlayModeResponse { bool is_playing = 1; }
message SetPlayModeRequest { bool play = 1; }
message SetPlayModeResponse { bool applied = 1; }
```

**proto/mcp/unity/v1/assets.proto**
```proto
syntax = "proto3";
package mcp.unity.v1;

service Assets {
  rpc ImportAsset(ImportAssetRequest) returns (ImportAssetResponse);
}

message ImportAssetRequest { string path = 1; }
message ImportAssetResponse { bool queued = 1; string op_id = 2; }
```

**proto/mcp/unity/v1/build.proto**
```proto
syntax = "proto3";
package mcp.unity.v1;

service Build {
  rpc BuildPlayer(BuildPlayerRequest) returns (BuildPlayerResponse);
}

message BuildPlayerRequest { string target = 1; /* e.g., "Android" */ }
message BuildPlayerResponse { bool started = 1; string op_id = 2; }
```

**proto/mcp/unity/v1/operations.proto**
```proto
syntax = "proto3";
package mcp.unity.v1;
import "mcp/unity/v1/common.proto";

service Operations {
  rpc GetOperation(OperationGetRequest) returns (OperationGetResponse);
  rpc CancelOperation(OperationCancelRequest) returns (OperationCancelResponse);
}

message OperationGetRequest { string id = 1; }
message OperationGetResponse { string id = 1; string state = 2; string message = 3; }

message OperationCancelRequest { string id = 1; }
message OperationCancelResponse { bool accepted = 1; }
```

**proto/mcp/unity/v1/events.proto**
```proto
syntax = "proto3";
package mcp.unity.v1;
import "mcp/unity/v1/common.proto";

service Events {
  // Server streaming example (bridge -> client). The Rust side will be a client receiving the stream.
  rpc SubscribeOperation(OperationRef) returns (stream OperationEvent);
}

message OperationEvent {
  string id = 1;     // operation id
  string kind = 2;   // e.g., "progress", "completed", "error"
  string payload = 3;// free-form JSON string at L0
}
```
**Acceptance:** No unresolved imports; `package mcp.unity.v1;` in all files.

---

## Step 4 — Configure code generation (server/build.rs)
Create `server/build.rs` with **exactly**:
```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protos = &[
        "proto/mcp/unity/v1/common.proto",
        "proto/mcp/unity/v1/editor_control.proto",
        "proto/mcp/unity/v1/assets.proto",
        "proto/mcp/unity/v1/build.proto",
        "proto/mcp/unity/v1/operations.proto",
        "proto/mcp/unity/v1/events.proto",
    ];

    tonic_build::configure()
        .build_server(false) // Rust side = gRPC client only
        .compile(protos, &["proto"]) ?;

    println!("cargo:rerun-if-changed=proto");
    Ok(())
}
```
**Notes**
- We **do not** set a custom `out_dir`. `include_proto!` will read from Cargo’s `OUT_DIR`.
- If you later add a proto, append it to `protos` and re-run build.

**Acceptance:** Running `cargo build -p server` triggers codegen (first build only).

---

## Step 5 — Wire generated stubs (server/src/main.rs)
Replace `server/src/main.rs` with **exactly**:
```rust
use anyhow::Result;

// Maps to files generated in OUT_DIR by tonic-build. Keep package path identical to .proto.
pub mod mcp_unity_v1 {
    pub mod editor_control { tonic::include_proto!("mcp.unity.v1"); }
}

// You can include more modules if you want separated namespaces; minimum demo uses EditorControl only.

#[tokio::main]
async fn main() -> Result<()> {
    // Just prove the types exist and the binary links; connection can fail if bridge is not running.
    // When the bridge is available at 127.0.0.1:50051, this will connect.
    let _ = tonic::transport::Channel::from_static("http://127.0.0.1:50051");

    println!("gRPC client stubs compiled and binary runs.");
    Ok(())
}
```
**Option (granular includes):** For distinct namespaces, use multiple `include_proto!` blocks with unique Rust modules:
```rust
pub mod editor_control { tonic::include_proto!("mcp.unity.v1"); }
pub mod assets         { tonic::include_proto!("mcp.unity.v1"); }
// …same package path, separate Rust modules if desired.
```
**Acceptance:** `cargo run -p server` prints: `gRPC client stubs compiled and binary runs.`

---

## Step 6 — .gitignore and repository hygiene
Create or update `repo-root/.gitignore`:
```
/target
**/*.rs.bk
.DS_Store
```
**Policy:** Do **not** commit generated code from `OUT_DIR` (Cargo target dir). Source of truth is `proto/` + `build.rs`.

---

## Step 7 — Local build and verification
```bash
cd repo-root
cargo clean && cargo build -p server -v
cargo run -p server
```
**Acceptance:**
- Build succeeds without warnings/errors.
- Run prints the success message. No runtime connection is required at this stage.

---

## Step 8 — Minimal CI (GitHub Actions)
Create `.github/workflows/rust-ci.yml`:
```yaml
name: Rust CI
on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install protoc
        run: |
          sudo apt-get update
          sudo apt-get install -y protobuf-compiler
          protoc --version
      - uses: dtolnay/rust-toolchain@stable
      - name: Build server
        run: cargo build -p server --locked --verbose
```
**Acceptance:** CI job passes and shows `protoc --version` output followed by successful build.

---

## Step 9 — Change management guardrails
**Until Schema Freeze**
- Breaking changes permitted to keep design clean.
- Coordinate updates via PRs labeled `proto:breaking`.
- Require consumers to regenerate (`cargo clean && cargo build -p server`).

**After Schema Freeze**
- Treat field numbers as wire contracts; never reuse or renumber.
- Use `reserved` for removed fields/ids; prefer additive changes.
- Maintain `docs/PROTO_CHANGELOG.md` for all schema edits.
- No googleapis at L0; reconsider in a future `v2` only if needed.
- Streaming payloads remain simple (strings/bytes) unless stabilized types are agreed.

---

## Step 10 — Troubleshooting matrix
| Symptom | Cause | Fix |
|---|---|---|
| `protoc: command not found` | Protobuf not installed or PATH not exported | Install `protobuf-compiler` (Ubuntu) / `brew install protobuf` (macOS). Re-run shell to refresh PATH. |
| `file not found: mcp/unity/v1/common.proto` | Include path not set | Ensure `compile(protos, &["proto"])` in `build.rs` and path spelling matches. |
| `duplicate symbol` / `conflicting types` | Package names or message names collide | Keep **one** `package mcp.unity.v1;` per file; avoid duplicate message names unless intended. |
| `include_proto!("mcp.unity.v1")` cannot find file | Custom `out_dir` set or package mismatch | Do **not** set `out_dir`. Ensure `package` exactly `mcp.unity.v1`. Clean/rebuild. |
| CI fails but local passes | CI missing `protoc` | Add explicit `protobuf-compiler` install step. |

---

## Completion criteria (Definition of Done)
1. `cargo build -p server` succeeds on macOS and Ubuntu.
2. `cargo run -p server` prints the success message.
3. CI (`rust-ci.yml`) passes on a clean clone.
4. All six proto files exist and compile; no googleapis imports.
5. Subsequent proto changes only require editing `.proto` + `build.rs` list (no other code changes).

---

## Next steps (out of scope here)
- Implement actual client calls using the generated `*_client` types once the Bridge server is live.
- Add contract tests against a mock Bridge (optional, later).

