# Step 0 — Replace gRPC codegen with **message-only** Protobuf generation (and keep the build green)

**Objective:** Remove gRPC from the build pipeline while keeping Protocol Buffers as the schema SSoT. After this step:
- No `tonic`/gRPC code is generated or required to compile.
- The crate still **builds successfully** on Windows/macOS/Linux.
- gRPC-dependent codepaths and tests are **feature-gated** (temporarily disabled).

This sets the stage for Step 1 (introducing the IPC layer) without fighting build breaks.

---

## 0) Preconditions & Assumptions
- Repo structure:
  - `proto/mcp/unity/v1/*.proto` exists.
  - Rust crate is at `server/`.
- Tools installed locally and on CI:
  - `protoc >= 3.21` (or any version your `prost-build` supports)
  - Rust stable (Tokio-compatible)

---

## 1) Dependencies — add via `cargo add` (no version pinning)

We avoid hardcoding versions and rely on Cargo.lock for reproducibility. Run the following in the `server/` crate:

```bash
# Remove gRPC stacks (if previously present)
cargo remove tonic tonic-build tonic-prost || true

# Core deps for IPC + Protobuf messages
cargo add anyhow
cargo add bytes
cargo add prost
cargo add tokio --features "macros,rt-multi-thread,net,io-util,time"
cargo add tokio-util --features "codec"

# Optional: unified IPC abstraction for UDS/Named Pipe
cargo add interprocess --features "tokio" --optional

# Build-time codegen for messages only
cargo add --build prost-build
```

**Notes**
- These commands enable *dependency features* only; your crate-level feature flags (e.g., `transport-ipc` / `transport-grpc`) remain unchanged. If you keep `transport-grpc` temporarily, guard gRPC modules with `#[cfg(feature = "transport-grpc")]` as described in Section 4.
- Commit `Cargo.lock` to lock resolved versions.

--- remove these (were used for gRPC) ---
# tonic = "*"
# tonic-build = "*"
# tonic-prost = "*"

[build-dependencies]
prost-build = "0.14"
```

**Notes**
- Keep `tokio` and `tokio-util` now; they will be used for IPC framing in Step 1.
- Leave `transport-grpc` feature in place for *temporary* compatibility (we disable it by default and will delete it in Step 6 of the overall plan).

---

## 2) build.rs — switch to prost-build (messages only)

Replace any `tonic_build` usage with plain `prost_build`. Prefer a **deterministic path** under `src/generated` so the code can be `include!`d with stable filenames.

```rust
// server/build.rs
use std::{env, fs, path::PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let proto_root = manifest_dir.join("..").join("proto");

    let files = [
        "mcp/unity/v1/common.proto",
        "mcp/unity/v1/editor_control.proto",
        "mcp/unity/v1/assets.proto",
        "mcp/unity/v1/build.proto",
        "mcp/unity/v1/operations.proto",
        "mcp/unity/v1/events.proto",
    ]
    .into_iter()
    .map(|rel| proto_root.join(rel))
    .collect::<Vec<_>>();

    let out_dir = manifest_dir.join("src").join("generated");
    fs::create_dir_all(&out_dir).unwrap();

    println!("cargo:rerun-if-changed={}", proto_root.display());

    let mut config = prost_build::Config::new();
    config.out_dir(&out_dir);

    // IMPORTANT: We are NOT generating any gRPC services here.
    config.compile_protos(
        &files.iter().map(PathBuf::as_path).collect::<Vec<_>>(),
        &[proto_root.as_path()],
    ).unwrap();
}
```

> If you prefer `OUT_DIR`, ensure you `include!(concat!(env!("OUT_DIR"), "/file.rs"))`. Using `src/generated/` is simpler for cross-editor workflows.

---

## 3) Module wiring — include generated messages

Replace any `tonic::include_proto!` usage. Create a stable module path for generated types.

```rust
// server/src/lib.rs (or a dedicated mod, e.g., server/src/generated/mod.rs)
#![allow(clippy::derive_partial_eq_without_eq)]

pub mod generated {
    pub mod mcp {
        pub mod unity {
            pub mod v1 {
                include!("generated/mcp.unity.v1.rs");
            }
        }
    }
}
```

> The exact filename under `src/generated/` is chosen by `prost-build`. With the config above, it typically flattens to `mcp.unity.v1.rs`.

---

## 4) Feature-gate existing gRPC codepaths (temporary)

**Goal:** Keep the crate building **without** gRPC. Deactivate modules and tests that rely on `tonic`.

### 4.1 Gate gRPC modules
Wrap the gRPC-specific modules with `#[cfg(feature = "transport-grpc")]`.

```rust
// server/src/grpc/mod.rs
#![cfg(feature = "transport-grpc")]

pub mod channel;     // existing
pub mod clients;     // existing
```

If a public API was re-exported from `grpc`, hide or alias it behind the same feature.

### 4.2 Guard call sites
Wherever gRPC clients are constructed/used (e.g., in tool handlers), add conditional compilation.

```rust
#[cfg(feature = "transport-grpc")]
use crate::grpc::clients::EditorControlClient;

#[cfg(feature = "transport-grpc")]
fn make_health_call(/*...*/) { /* existing gRPC impl */ }

#[cfg(not(feature = "transport-grpc"))]
fn make_health_call(/*...*/) {
    // Temporary stub: return a clear error until IPC (Step 1/2) lands.
    // Use anyhow::bail! or a custom error type.
    // This keeps the build green without gRPC.
}
```

### 4.3 Gate integration tests
Disable tests that spin up a gRPC server/client.

```rust
// tests/health_mcp.rs
#![cfg(feature = "transport-grpc")] // until IPC test server arrives in Step 2
```

> Optional: introduce `#[cfg(feature = "transport-ipc")]` tests later.

---

## 5) Codebase search & cleanup checklist
- [ ] Replace any `tonic::include_proto!` occurrences with `include!("generated/…")`.
- [ ] Remove imports from `tonic`/`tonic::transport` in non-gated files.
- [ ] Ensure **no module** that is compiled by default references `tonic` symbols.
- [ ] Delete obsolete `build.rs` gRPC bits (`tonic_build`, service generation options).
- [ ] Add `#![allow(clippy::derive_partial_eq_without_eq)]` on the generated module if Clippy complains.

---

## 6) Verification steps
1. **Clean state**
   ```bash
   cd server
   cargo clean
   ```
2. **Build (default features = transport-ipc)**
   ```bash
   cargo build --verbose
   ```
   - Expect: success without pulling `tonic*` crates.
3. **Feature matrix sanity**
   ```bash
   cargo tree -e features | rg -i tonic || echo "OK: tonic not in tree"
   cargo check --no-default-features --features transport-grpc   # should build if you still need it for legacy tests
   ```
4. **CI**: ensure the workflow uses the same commands as above and does **not** install any gRPC-specific tooling.

---

## 7) Troubleshooting
- **`protoc` not found / version too old** → install `protobuf-compiler` (Linux) or `brew install protobuf` (macOS) / `choco install protoc` (Windows). Re-run build.
- **`include!("generated/…")` not found** → check `build.rs` output dir; verify the exact file name under `src/generated/`.
- **Residual references to gRPC types** → search for `tonic`, `EditorControlClient`, `ChannelManager`, `transport::Channel` and either guard or remove.
- **Clippy warnings from generated code** → add `#![allow(...)]` at the generated mod boundary.

---

## 8) Rollback plan (fast)
- Revert `Cargo.toml` and `build.rs` to the gRPC-enabled versions.
- Re-enable the `transport-grpc` feature by default if you must unblock a release.

---

## 9) Definition of Done (Step 0)
- `cargo build` passes on all platforms with **default features** (IPC track) and without any `tonic*` in the dependency graph.
- gRPC-dependent code and tests are **present but disabled** behind `transport-grpc`.
- Generated message types compile and are importable via the `generated::mcp::unity::v1::*` path.

---

## 10) What’s next (Step 1 preview)
- Add `server/src/ipc/{path.rs,framing.rs,codec.rs,client.rs}`.
- Implement connection, handshake, and request/response plumbing using `tokio` + `tokio-util` + `prost`.
- Replace temporary stubs (Section 4.2) with real IPC calls.

