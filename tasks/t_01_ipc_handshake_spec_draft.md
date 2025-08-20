# T01 — IPC Handshake Spec (Draft)

**Status:** Draft for MVP freeze

**Objective** Define and implement a deterministic, secure, and forward‑compatible handshake between the Rust MCP server (client role on IPC) and the Unity Editor bridge (server role on IPC). After a successful handshake, both sides may exchange regular IPC envelopes and events.

---

## 1. Design Summary

- **Transport:** length‑prefixed binary frames over the chosen IPC transport (TCP localhost for MVP). Length prefix is an unsigned 32‑bit little‑endian integer `N`, followed by `N` bytes of payload.
- **Handshake frames:** two control frames before any regular envelope:
  1. `IpcControl{ hello: IpcHello }` from Rust → Unity
  2. `IpcControl{ welcome: IpcWelcome }` from Unity → Rust
  - On failure, Unity replies with `IpcControl{ reject: IpcReject }` and closes.
- **Versioning:** semantic version string `ipc_version` (e.g. `1.0`). Major must match; minor may differ (feature negotiation decides capability).
- **Auth:** pre‑shared token. Required in MVP.
- **Feature negotiation:** Rust proposes features; Unity returns the accepted subset.
- **Schema lock:** both sides present a hash of the compiled proto descriptor set. If mismatch, return `FAILED_PRECONDITION` in `reject`.

---

## 2. Protobuf (ipc\_control.proto)

> Ship as a new file under `proto/mcp/unity/v1/ipc_control.proto` and add it to builds on both sides.

```proto
syntax = "proto3";
package mcp.unity.v1;

message IpcControl {
  oneof kind {
    IpcHello hello = 1;
    IpcWelcome welcome = 2;
    IpcReject reject = 3;
  }
}

message IpcHello {
  // Security
  string token = 1;                    // required

  // Protocol compatibility
  string ipc_version = 2;              // e.g. "1.0"; major must match
  repeated string features = 3;        // requested feature flags (see §6)

  // Schema & environment
  bytes schema_hash = 4;               // SHA-256 of FileDescriptorSet (see §4)
  string project_root = 5;             // absolute path; normalized
  string client_name = 6;              // e.g. "unity-mcp-rs"
  string client_version = 7;           // semver of Rust server

  map<string,string> meta = 8;         // optional free-form (OS, arch, etc.)
}

message IpcWelcome {
  // Echoed/negotiated
  string ipc_version = 1;              // server-supported for this session
  repeated string accepted_features = 2;
  bytes schema_hash = 3;               // server view of schema

  // Server info
  string server_name = 4;              // e.g. "unity-editor-bridge"
  string server_version = 5;           // plugin/package version
  string editor_version = 6;           // e.g. "Unity 6000.0.x"
  string session_id = 7;               // UUID for logs and tracing

  map<string,string> meta = 8;         // optional (platform, license, etc.)
}

message IpcReject {
  enum Code {
    UNAUTHENTICATED = 0;
    FAILED_PRECONDITION = 1;  // schema mismatch, editor state invalid
    PERMISSION_DENIED = 2;    // token valid but insufficient rights
    OUT_OF_RANGE = 3;         // unsupported major version
    INTERNAL = 4;             // unexpected error
    UNAVAILABLE = 5;          // editor busy starting up, try later
  }
  Code code = 1;
  string message = 2;         // single-sentence reason
}
```

**Wire order:** `IpcControl(hello)` → `IpcControl(welcome)` or `IpcControl(reject)` → (then switch to normal `IpcEnvelope` traffic).

---

## 3. State Machine

```
Client(Rust)                              Server(Unity)
-----------------------------------------------------------------
CONNECT ---------------------------------> [ACCEPT]
SEND IpcControl(hello) ------------------>
                                          VERIFY token, version, schema, features
                                          if ok: SEND IpcControl(welcome)
                                          else : SEND IpcControl(reject); CLOSE
WAIT welcome <----------------------------
ON welcome: START normal traffic (envelopes/events)
```

**Timeouts**

- Connect timeout: 2s (dev) / 5s (CI)
- Handshake read timeout: 2s after sending `hello`
- Entire handshake must complete within 3s (dev) / 8s (CI). On timeout, abort with `UNAVAILABLE` and backoff.

**Reconnect policy (client)**

- Exponential backoff starting at 250ms, jitter ±25%, max 5s. Give up after N attempts if configured.

---

## 4. Schema Hash

**Goal:** detect client/server proto drift without depending on file timestamps.

**Definition:** `schema_hash = SHA-256( FileDescriptorSet )`, where `FileDescriptorSet` is produced by `protoc` with:

- inputs: **all** `proto/mcp/unity/v1/*.proto` actually used by the project (sorted by path)
- flags: `--include_imports` (true), `--include_source_info` (false)
- consistency: run with a stable `protoc` (≥ 3.21) across both sides

**Notes**

- This is resilient to whitespace and ordering differences in source as long as the compiled descriptors are equivalent.
- If hash mismatches: Unity replies `FAILED_PRECONDITION` with message `"schema_hash mismatch; client=… server=…"`.

---

## 5. Security

- **Token requirement:** MVP requires a non-empty token. Unity compares against its configured value.
- **Configuration**
  - Rust: env `MCP_IPC_TOKEN` (or CLI flag `--ipc-token`); project root sent from Rust is validated by Unity PathPolicy.
  - Unity: token stored via EditorPrefs/ProjectSettings or env `MCP_IPC_TOKEN` (dev mode). Never log raw token.
- **Failure semantics:** invalid/missing token → `UNAUTHENTICATED`; do not disclose expected value; close connection.
- **PathPolicy precheck:** during handshake, Unity may compute and cache canonical `project_root`; if outside the actual project folder, reject with `FAILED_PRECONDITION`.

---

## 6. Feature Flags (strings)

Use dotted identifiers. Initial set:

- `assets.basic` — path↔guid, import, refresh
- `build.min` — minimal player build with operation events
- `events.log` — Unity log events
- `ops.progress` — generic progress events for long‑running operations

**Negotiation rule:** client proposes subset; server returns accepted subset. Client must degrade gracefully if a requested feature is not accepted.

---

## 7. Error Mapping

Handshake errors use `IpcReject.Code` (see §2). After handshake, normal requests map editor failures to the common vocabulary (documented elsewhere). Messages must be **single sentence**, actionable where possible.

Examples

- Token missing → `UNAUTHENTICATED: "missing token"`
- Major version mismatch → `OUT_OF_RANGE: "ipc_version 2.x not supported; server=1.x"`
- Schema mismatch → `FAILED_PRECONDITION: "schema_hash mismatch; client=… server=…"`
- Editor not ready → `UNAVAILABLE: "editor starting up"`

---

## 8. Backward/Forward Compatibility Rules

- **Major version:** must match exactly.
- **Minor version:** server may accept older/newer minor as long as all `accepted_features` are supported.
- **Unknown features:** silently dropped from `accepted_features`.
- **Unknown fields in protobuf:** ignored per proto3 rules.

---

## 9. Implementation Notes

### Rust (client role)

- Location: `server/src/ipc/client.rs`
- Steps:
  1. Connect (TCP/UDS later). Set `TCP_NODELAY`.
  2. Compose and send `IpcControl(hello)`.
  3. Read a single frame and decode to `IpcControl`.
  4. If `welcome`: validate echoed items, stash `session_id`/features; transition to envelope mode.
  5. If `reject` or decode error: return rich error; trigger backoff per policy.
- Logging: `info!` on success (version, features, short hash), `warn!` on transient failures, `error!` on auth/config errors.

### Unity (server role)

- Location: `Editor/Runtime/IpcServer/*`
- Steps:
  1. Accept connection; read exactly one control frame.
  2. Verify token and version; compute schema hash lazily or cache.
  3. Intersect features; construct `welcome` with `session_id` (GUID) and editor/plugin versions.
  4. On failure: send `reject` with appropriate `code` and one‑line message; close.

---

## 10. Test Plan

- **Unit**
  - Protobuf round‑trip for all control messages.
  - Framing/deframe edge cases (split frames, overlong length, empty payload).
  - Schema hash computation from a fixture `FileDescriptorSet`.
- **Integration**
  - Success handshake: valid token + matching schema.
  - Invalid token → `reject(UNAUTHENTICATED)`.
  - Version mismatch (major) → `reject(OUT_OF_RANGE)`.
  - Schema drift → `reject(FAILED_PRECONDITION)`.
  - Editor not ready path → `reject(UNAVAILABLE)`.
- **Soak**
  - 100 connects/disconnects; ensure no leaks, stable CPU.

**Acceptance (DoD for T01)**

- Rust logs `Handshake OK: version=1.0, features=[…], schema=<8 hex>, session=<uuid>`.
- All negative paths above produce expected `IpcReject` and close.

---

## 11. Pseudocode

### Rust

```rust
let stream = connect_with_timeout(endpoint, Duration::from_secs(2))?;
let hello = IpcControl { kind: Some(ipc_control::Kind::Hello(IpcHello {
    token,
    ipc_version: "1.0".into(),
    features: vec!["assets.basic".into(), "events.log".into(), "build.min".into()],
    schema_hash,
    project_root,
    client_name: "unity-mcp-rs".into(),
    client_version: env!("CARGO_PKG_VERSION").into(),
    meta: default_meta(),
}))};
send_frame(&mut stream, &hello)?;
let ctrl: IpcControl = read_frame_with_timeout(&mut stream, Duration::from_secs(2))?;
match ctrl.kind {
  Some(ipc_control::Kind::Welcome(w)) => validate_and_switch(w),
  Some(ipc_control::Kind::Reject(r)) => bail!(format!("{:?}: {}", r.code, r.message)),
  _ => bail!("unexpected control frame"),
}
```

### Unity (C#)

```csharp
var ctrl = ReadControlFrame();
if (ctrl.Hello is null) { return Reject(Code.INTERNAL, "expected hello"); }
if (!ValidateToken(ctrl.Hello.Token)) { return Reject(Code.UNAUTHENTICATED, "invalid token"); }
if (!IsMajorSupported(ctrl.Hello.IpcVersion)) { return Reject(Code.OUT_OF_RANGE, $"ipc_version {ctrl.Hello.IpcVersion} not supported"); }
if (!SchemaMatches(ctrl.Hello.SchemaHash)) { return Reject(Code.FAILED_PRECONDITION, "schema_hash mismatch"); }
var accepted = IntersectFeatures(ctrl.Hello.Features);
var welcome = new IpcWelcome {
  IpcVersion = NegotiatedVersion(ctrl.Hello.IpcVersion),
  AcceptedFeatures = { accepted },
  SchemaHash = LocalSchemaHash(),
  ServerName = "unity-editor-bridge",
  ServerVersion = Package.Version,
  EditorVersion = Application.unityVersion,
  SessionId = Guid.NewGuid().ToString(),
  Meta = { {"platform", Application.platform.ToString()} }
};
SendControlFrame(new IpcControl { Welcome = welcome });
SwitchToEnvelopeMode();
```

---

## 12. Dev/Config Defaults (MVP)

- `ipc_version`: `1.0`
- `features` (Rust): `assets.basic`, `events.log`, `build.min`, `ops.progress`
- Env vars (Rust): `MCP_IPC_TOKEN`, `MCP_ENDPOINT` (e.g., `tcp://127.0.0.1:7777`), `MCP_PROJECT_ROOT`
- Env vars (Unity, optional dev): `MCP_IPC_TOKEN`
