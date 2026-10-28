# AETHER family: official identity and integration contract

Status: **proposed architecture decision for review** (2026-10). This document records product naming and intended boundaries; it does not claim cross-repository integration or tests already exist.

## Official product names

- **AETHER** — the modular Rust application framework and composition layer. It combines application lifecycle, module orchestration, configuration, events, background tasks and policy-controlled AI capabilities where enabled.
- **ATLAS** — the Rust frontend framework for browser interfaces compiled to WebAssembly (WASM).
- **TITAN** — the Rust backend HTTP framework for APIs and services.

These are the public product/repository names for the family. The names currently used inside the codebases — **Liga** (`liga-*`), **Patina** (`patina-*`) and **Ferrovia** (`ferrovia-*`) — are treated as existing internal/package identities during a compatibility transition. Do not rename Cargo packages, Rust crate imports, environment-variable prefixes, examples or public symbols as part of this documentation-only decision. Any rename requires a separate migration plan, impact inventory and explicit approval.

## AETHER's role

AETHER is the **composition framework**, not a mandatory dependency for every application. It may coordinate modules, lifecycle, configuration, events, tasks and optional AI policy. It should consume explicit integration adapters/contracts rather than reaching into ATLAS or TITAN internals.

AETHER must remain usable without ATLAS. Its core must not acquire browser/WASM dependencies merely to support the family. Optional integrations should be feature-gated and tested independently.

## Integration model

The intended application flow is:

`ATLAS (browser/WASM UI) → versioned HTTP/JSON contract → TITAN (API/service)`

AETHER is optional at the application-composition layer. It can host application modules and background/AI workflows alongside a TITAN service, or integrate through a documented adapter. This is an architectural target, not a statement that these adapters currently exist.

### Contract rules

1. **No circular dependencies.** ATLAS must not depend on TITAN or AETHER internals. TITAN must not depend on ATLAS. AETHER may depend on stable public contracts or optional adapters, never private implementation details.
2. **Transport boundary.** Frontend/backend communication uses documented HTTP methods, paths, JSON schemas and status codes. Do not share Rust structs across repositories as an implicit wire format.
3. **Versioning.** Public API changes must be documented. Prefer additive changes within a compatible API version; breaking changes require a versioning/migration decision.
4. **Errors.** Agree on a documented JSON error envelope, stable machine-readable error codes and a request/correlation identifier. Map each framework's existing behavior before claiming full compatibility.
5. **Security.** CORS is not authentication. Production deployments must explicitly define authentication/authorization, secret handling, input limits, timeouts, logging redaction and deployment TLS responsibilities.
6. **Observability.** Carry a request/correlation ID from frontend diagnostics into backend requests when available; never put secrets or sensitive payloads in logs.
7. **Optionality.** Each framework remains independently usable. Integration must not force a user to adopt all three.
8. **No hidden feature claims.** An item remains “planned” until code, tests and documentation demonstrate it.

## Compatibility and crate naming

- Product/repository brand: AETHER.
- Current package namespace: `liga-*` (observed in the workspace manifest).
- Public Rust API: existing crate names and symbols remain unchanged until a separate approved migration.
- A future rename must inventory imports, examples, docs, feature names, environment variables, lockfiles and downstream users; provide migration notes and a deprecation window where practical.

## Delivery sequence

1. Review this identity and boundary proposal.
2. Record the accepted decision as an ADR.
3. Map current code to the intended contracts; mark each item implemented, partial, absent or unverified with file/test evidence.
4. Specify a minimal HTTP contract and error-envelope compatibility matrix.
5. Implement adapters only after contracts and tests are agreed.
6. Consider crate/package renaming separately, after identity approval.

## Current evidence and limits

The current README describes AETHER's workspace as `liga-core`, `liga-modules`, `liga-web`, `liga-tasks`, `liga-ai` and `liga`. This document does not re-run or independently validate the README's reported test results. No integration with ATLAS or TITAN is asserted here.

## Decision record

- Proposed: AETHER = composition/application framework; ATLAS = frontend; TITAN = backend HTTP.
- Proposed: keep the three projects independently usable with one-way, documented contracts.
- Deferred: changing crate names, creating a shared repository/workspace, and declaring a unified stable API.
- Approval: pending maintainer review.
