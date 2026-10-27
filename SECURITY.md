# Security Policy

## Supported versions

Until a stable release policy is published, security fixes should target the latest development version on the default branch. This is a project policy proposal and does not imply a production-support commitment.

## Reporting a vulnerability

Please do not publish exploit details in a public issue. Use GitHub's private vulnerability reporting or Security Advisories feature for this repository if available. If that feature is unavailable, contact the repository maintainers privately through their GitHub profile and include a minimal reproduction, affected revision, impact, and suggested mitigation where possible.

## Security review priorities

- Keep AI tool authorization deny-by-default and enforced by the host.
- Never log API keys, secret values, raw authorization headers, or sensitive tool output.
- Bound provider requests, tool arguments/results, execution time, retries, and concurrency.
- Treat model output and external tool results as untrusted input.
- Run the RustSec advisory audit against Cargo.lock in CI.
- Do not describe the framework as production-ready until security and operational limitations are documented and reviewed.
