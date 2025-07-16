# Unity MCP Server – MVP Roadmap

## Goal
Deliver a headless‑operable Unity MCP Server (MVP) that enables external tools and CI/CD pipelines to automate Unity Editor tasks reliably and securely.

## Deliverables
- **Transport & RPC Layer**: JSON‑RPC (initial) with a pluggable gRPC adapter.
- **Session & Authentication**: token‑based connection hand‑shake, concurrent client limits, timeout handling.
- **Asset Operations API**: import, move, delete, GUID query, and refresh using `AssetDatabase`.
- **Build Pipeline Control**: wrapper around `BuildPipeline.BuildPlayer` supporting target platform, build options, and output path.
- **PlayMode Control**: enter/exit PlayMode, query play state, and safe shutdown hooks.
- **Log & Error Streaming**: real‑time relay of `Application.logMessageReceivedThreaded` events with severity levels.
- **Developer Tooling**: unit tests, integration tests, and English documentation for setup and usage.

---

## Milestones

| Sprint | Focus | Key Outcomes |
|--------|-------|--------------|
| **0** | Project Bootstrap | Initialize Git repo, add template project (`generate-project --name mcp`), set up CI skeleton (lint, build), configure coding standards. |
| **1** | Core Transport & Session | Implement stdio transport prototype, define JSON‑RPC schema, add token authentication, basic health‑check method. |
| **2** | Asset Subsystem | Expose asset import/move/delete endpoints, implement GUID lookup, write unit tests, update docs. |
| **3** | Build & PlayMode | Add build endpoint with platform args; implement PlayMode enter/exit and state query; sandbox tests. |
| **4** | Logging & E2E Tests | Integrate threaded log streaming, create end‑to‑end test suite that covers asset‑>build pipeline; document examples. |
| **5** | Hardening & Release Candidate | Load tests, error handling polish, API versioning, final docs, semantic release packaging. |

---

## Definition of Done
1. All deliverables implemented and covered by automated tests.
2. CI workflow passes on clean repository checkout.
3. English developer guide explains installation, configuration, and sample calls.
4. Release candidate tag pushed and binary package published to internal registry.

## Out of Scope (MVP)
- Addressables, Lightmapping, Profiler data access, and Live‑Link features.
- Collaboration / multi‑user editing capabilities.
- SSE or Streamable HTTP transport (planned for post‑MVP).

