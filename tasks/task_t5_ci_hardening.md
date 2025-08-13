# T5 — CI Hardening: Fixes & Final GitHub Actions Workflows

This document replaces the draft CI with a robust, cross‑platform setup. It pins `protoc`, uses a maintained Rust toolchain action, enables reliable caching, runs fmt/clippy/tests, and aligns with the earlier **server‑stubs** feature gate used in T4.

---

## What changes (and why)

1. **Pin `protoc` via an action**
   `apt-get install protobuf-compiler=3.21.*` is brittle across runners. Use `arduino/setup-protoc@v3` and pin to an exact version (e.g., `3.21.12`). Works on Linux/macOS/Windows without distro gymnastics.

2. **Use a maintained Rust setup**
   `actions-rs/toolchain@v1` is effectively unmaintained. Switch to `dtolnay/rust-toolchain@stable` which is the community standard and supports `components: rustfmt, clippy`.

3. **Smarter caching**
   Replace manual `actions/cache` blocks with `Swatinem/rust-cache@v2`. It handles `target/`, registry, and git cache consistently across workspace crates.

4. **Workspace‑root commands**
   Run `cargo fmt/clippy/build/test` from repo root with `--workspace` or `-p server` to avoid ambiguity (earlier draft used `working-directory: server`).

5. **Locked builds and clearer logs**
   Use `--locked` to respect `Cargo.lock`. Set `RUST_BACKTRACE=1` and `CARGO_TERM_COLOR=always` for actionable diagnostics.

6. **Feature‑gated server stubs**
   Prefer `--features server-stubs` over env vars to make intent explicit. (Env fallback `TONIC_BUILD_SERVER=1` still works but we standardize on the feature.)

7. **Concurrency and permissions**
   Add `concurrency` to cancel superseded runs and least‑privilege `permissions` in the workflow.

---

## Workflow consolidation (deprecate `rust-ci.yml`)

We will maintain a **single** workflow file going forward. Please remove the legacy `rust-ci.yml` and keep only `ci.yml`.

**Actions**

1. Add `.github/workflows/ci.yml` (the workflow defined in this document).
2. Remove `.github/workflows/rust-ci.yml` to prevent duplicate triggers.
3. Update repository settings → **Branch protection** → **Required status checks**:

   * Remove checks created by the old `rust-ci.yml`.
   * Add the new job name produced by this workflow (e.g., `Rust (build, lint, test) [ubuntu & macOS]`), or whatever name you set in this file.
   * The new check name becomes selectable after this workflow has run once.
4. Optional zero‑downtime cutover:

   * Temporarily change `rust-ci.yml` to `on: workflow_dispatch` only, **or**
   * Rename it to `rust-ci.yml.disabled` (GitHub Actions will ignore it), then delete after verifying `ci.yml`.

**Quick migration**

```bash
git rm .github/workflows/rust-ci.yml
git add .github/workflows/ci.yml
git commit -m "CI: consolidate into single workflow (remove rust-ci.yml)"
```

## Final workflow: `.github/workflows/ci.yml`

```yaml
name: CI

on:
  push:
    branches: [ main, develop ]
  pull_request:

permissions:
  contents: read

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

jobs:
  build-test:
    name: Build, Lint, Test (matrix)
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    env:
      CARGO_TERM_COLOR: always
      RUST_BACKTRACE: 1

    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Set up protoc 3.21.12
        uses: arduino/setup-protoc@v3
        with:
          version: "3.21.12"

      - name: Set up Rust (stable + fmt + clippy)
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Cache cargo (target/ + registries)
        uses: Swatinem/rust-cache@v2
        with:
          # Auto-detects workspace and caches target/ and registries
          # Customize if needed: cache-on-failure: true
          cache-on-failure: true

      - name: Show tool versions
        run: |
          rustc -V
          cargo -V
          protoc --version

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Lint (clippy)
        run: cargo clippy --workspace --all-targets -- -D warnings

      - name: Build (workspace, locked)
        run: cargo build --workspace --locked

      - name: Test server (with server stubs)
        run: cargo test -p server --features server-stubs -- --nocapture
```

> Notes
>
> * If you prefer env gating for server stubs, swap the test step for:
>
>   ```yaml
>   - name: Test server (env‑gated stubs)
>     env:
>       TONIC_BUILD_SERVER: "1"
>     run: cargo test -p server -- --nocapture
>   ```
> * For larger workspaces, add `-p other-crate` as needed or keep `--workspace` for tests as well.

---

## Optional: `clippy` lints without breaking dependencies

Avoid setting `RUSTFLAGS=-D warnings` globally (it denies warnings in dependencies). The step above only denies clippy warnings for your code. If you want to deny rustc warnings in **your** crates, configure `#![deny(warnings)]` at crate root or use `-Z` flags on nightly in CI (not recommended here).

---

## macOS local parity

### Install pinned protoc via Homebrew

```bash
brew install protobuf@21
# Prefer explicit pathing to avoid PATH ambiguity
export PROTOC="$(brew --prefix protobuf@21)/bin/protoc"
$PROTOC --version  # should match 3.21.x
```

### Verify Rust and run checks

```bash
rustup default stable
rustup component add rustfmt clippy

# From repo root
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --locked
# Server tests that need server stubs
cargo test -p server --features server-stubs -- --nocapture
```

---

## Acceptance checklist

* ✅ `protoc` is pinned to **3.21.12** in CI and reproducible locally (Homebrew path override).
* ✅ Cargo caching via `Swatinem/rust-cache@v2` speeds up subsequent runs.
* ✅ `fmt`, `clippy`, `build`, and `test` steps run on Linux and macOS.
* ✅ Server tests that require stub generation pass with `--features server-stubs` (or env fallback).
* ✅ Locked builds (`--locked`) ensure reproducible dependency resolution.

---

## Future hardening (optional)

* Add `cargo-nextest` for faster and more reliable test runs.
* Add `cargo-deny` to audit licenses and duplicate deps.
* Integrate `sccache` for bigger compile wins on CI.
* Split jobs (lint/build/test) and use `needs:` to parallelize.