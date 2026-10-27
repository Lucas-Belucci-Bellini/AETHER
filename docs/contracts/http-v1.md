# Family HTTP contract v1

Status: implementation in progress. This specification is the proposed shared wire contract for AETHER, ATLAS and TITAN. Do not assume every route in every framework conforms yet.

## Scope

- ATLAS consumes HTTP/JSON APIs from TITAN or other compatible servers.
- TITAN is the first reference implementation for versioned endpoints.
- AETHER can host application routes and orchestrate modules, but remains independently usable.
- Existing unversioned routes are not changed by this contract.

## Successful response envelope

Versioned successful endpoints return JSON shaped like:

\`\`\`json
{
  "data": {
    "message": "Olá do TITAN!",
    "environment": "development"
  },
  "meta": {
    "api_version": "v1"
  }
}
\`\`\`

Rules:
- \`data\` contains the route-specific response.
- \`meta.api_version\` is the literal API version, currently \`v1\`.
- Clients should ignore unknown additive fields unless a schema explicitly says otherwise.
- JSON field names are snake_case.
- Content type is \`application/json\`.

## Versioned error envelope

The wire contract for errors explicitly handled by a v1 route is:

\`\`\`json
{
  "error": {
    "code": "not_found",
    "message": "recurso não encontrado"
  },
  "meta": {
    "api_version": "v1"
  }
}
\`\`\`

- \`error.code\` is stable and intended for programmatic handling.
- \`error.message\` is safe display text; it must not contain stack traces, secrets, SQL, or internal diagnostics.
- The HTTP status remains authoritative. Clients must not infer success from the body alone.
- Frameworks may have additional legacy error fields outside this minimal contract. Do not assume every existing route has opted in.
- Request correlation uses the \`x-request-id\` response header when available.

## First reference endpoint

\`GET /api/v1/hello\`

Expected status: \`200 OK\`. The reference TITAN implementation returns \`data.message\`, \`data.environment\` and \`meta.api_version\`. The endpoint is a contract smoke test, not a production health or readiness probe.

## Security and compatibility

- Use HTTPS in production deployments.
- CORS is not authentication or authorization.
- Document authentication requirements per endpoint before adding protected endpoints.
- Additive response fields are preferred; incompatible changes require a new version or migration plan.
- No Rust type is shared across repositories as an implicit wire contract.

## Implementation status

- AETHER \`liga-web\` now exposes typed success and error-envelope wire types. Existing global \`ApiError\` responses are not automatically rewritten.
- TITAN \`GET /api/v1/hello\` is the reference success endpoint; versioned 404/405 handling is being added separately from legacy routes.
- ATLAS has a browser client; only versioned error envelopes should be interpreted using the family-specific \`error.code\` / \`error.message\` fields.
- Cross-repository adapter/conformance tests and browser-level verification remain required before production use.
