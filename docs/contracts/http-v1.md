# Family HTTP contract v1

Status: implementation in progress. This specification is the proposed shared wire contract for AETHER, ATLAS and TITAN. Do not assume every route in every framework conforms yet.

## Scope

- ATLAS consumes HTTP/JSON APIs from TITAN or other compatible servers.
- TITAN is the first reference implementation for versioned endpoints.
- AETHER can host application routes and orchestrate modules, but remains independently usable.
- Existing unversioned routes are not changed by this contract.

## Successful response envelope

Versioned endpoints should return JSON shaped like:

```json
{
  "data": {
    "message": "Olá do TITAN!",
    "environment": "development"
  },
  "meta": {
    "api_version": "v1"
  }
}
```

Rules:
- `data` contains the route-specific response.
- `meta.api_version` is the literal API version, currently `v1`.
- Clients must ignore unknown fields for forward-compatible additions unless a schema says otherwise.
- JSON field names are snake_case.
- Content type is `application/json`.

## First reference endpoint

`GET /api/v1/hello`

Expected status: `200 OK`.

The reference TITAN implementation returns `data.message`, `data.environment` and `meta.api_version`. The endpoint is a contract smoke test, not a production health or readiness probe.

## Errors and request IDs

Error envelopes are not yet unified across all three codebases. Existing AETHER and TITAN error formats differ. Until an explicit compatibility change is reviewed:
- clients must handle non-2xx status codes without assuming one exact error body;
- clients may use `x-request-id` for support diagnostics when supplied;
- never log credentials or sensitive request/response bodies.

## Security and compatibility

- Use HTTPS in production deployments.
- CORS is not authentication or authorization.
- Document authentication requirements per endpoint before adding protected endpoints.
- Additive response fields are preferred; incompatible changes require a new version or migration plan.
- No Rust type is shared across repositories as an implicit wire contract.

## Implementation status

- Contract documentation: proposed in all three repositories.
- TITAN `GET /api/v1/hello`: added in the coordinated branch with a dedicated integration test; test execution is pending.
- AETHER `liga-web` exposes `ApiEnvelope<T>` and `ApiMeta`, with a serialization unit test added; it is not yet used by every existing route.
- ATLAS browser client: added in the coordinated branch; browser/WASM verification is pending.
- AETHER-to-TITAN adapter and cross-repository conformance tests: still absent.
- Tests/builds: not run by the authoring action; verify through CI or local execution before claiming success.
