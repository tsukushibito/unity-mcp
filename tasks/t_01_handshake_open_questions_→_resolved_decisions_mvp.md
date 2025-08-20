# T01 — Handshake Open Questions → Resolved Decisions (MVP)

**Purpose**
This document isolates the former “Open Questions” from the T01 Handshake Spec and records the **resolved decisions** for the MVP, with rationale and implementation notes.

---

## 1) Anonymous / No‑Token Dev Mode
**Decision:** **Not allowed** in MVP — a non‑empty token is always required.

**Rationale**
- Prevents accidental cross‑process/foreign‑editor attachment on a shared machine.
- Makes failure modes crisp (auth is an invariant), improving debuggability.
- Keeps configuration uniform for local dev and CI (no extra branching paths).

**Implementation Notes**
- Rust: supply via env `MCP_IPC_TOKEN` or CLI `--ipc-token`.
- Unity: store token in ProjectSettings/EditorPrefs (or env for dev); never log the token value.
- Failure: respond `IpcReject.UNAUTHENTICATED` with a single‑sentence reason, then close.

---

## 2) `project_root` in `IpcHello`
**Decision:** **Required**. Unity validates it against the actual project root (canonicalized).

**Rationale**
- Provides an explicit anchor for PathPolicy (prevents traversal/out‑of‑project ops).
- Disambiguates multi‑project and multi‑editor scenarios; future‑proofs remote/CI.
- Aids diagnostics by surfacing agreed context in logs.

**Implementation Notes**
- Canonicalization: resolve symlinks; normalize separators and case (Windows drive letter uppercase); remove trailing separators.
- On mismatch: `IpcReject.FAILED_PRECONDITION` with message `"project_root mismatch (client=… server=…)"`, then close.

---

## 3) `schema_hash` Mismatch Policy
**Decision:** **Reject on mismatch** in MVP (no degraded/"warn‑only" mode).

**Rationale**
- Proto3’s unknown‑field semantics risk silent corruption (defaulted values, enum drift).
- Forces regeneration/config fixes to surface immediately (prevents heisenbugs).
- Keeps the handshake invariant simple: post‑handshake traffic assumes a shared schema.

**Implementation Notes**
- Hash = SHA‑256 over the `FileDescriptorSet` compiled with `--include_imports`, `--include_source_info=false`, stable `protoc ≥ 3.21`.
- Reject with `IpcReject.FAILED_PRECONDITION` and short hashes in the message: `"schema_hash mismatch; client=abcd1234 server=ef567890"`.
- MVP does **not** allow a “health‑only” degraded session or a `--force` flag.

---

## Test Checklist (Handshake Decisions)
- Missing/empty token → `UNAUTHENTICATED`, connection closed.
- Non‑canonical or foreign `project_root` → `FAILED_PRECONDITION`, closed.
- Descriptor drift (different build of protos) → `FAILED_PRECONDITION`, closed.
- Happy path logs include: ipc_version, accepted_features, short schema hash, session_id.

---

## Change Impact
- No protobuf changes required beyond the existing `IpcControl` messages; semantics clarified.
- T01 Handshake Spec should reference this page as normative for MVP behavior.

