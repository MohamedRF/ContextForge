# Security

This is an unsigned developer preview build, not a hardened service. Use only local stdio. No telemetry, network service, source execution or source upload is implemented. Repository source contents are untrusted data. See [the security model](docs/security.md) for filesystem/cache/MCP trust boundaries, resource limits, cancellation behavior, permissions and remaining gaps.

## Reporting a vulnerability

Contact the maintainer privately at **soft-arch@ots.annauniv.edu**. Include the affected commit, operating system, a minimal reproduction using synthetic data, expected behavior, and the security impact. Do not include credentials or private source code, and do not publish exploit details in a public issue before coordination.

Only the current development branch is maintained; the published desktop preview is unsigned and does not establish a supported release line. Response and fix timelines depend on maintainer availability. Avoid running this development build on actively hostile, concurrently modified directories or attacker-controlled caches.
