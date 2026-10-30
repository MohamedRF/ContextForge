# Security

This is an unreleased development build, not a hardened service. Use only local stdio. No telemetry, network service, source execution or source upload is implemented. Repository source contents are untrusted data. See docs/architecture.md for the filesystem threat model and remaining race/cancellation limits.

## Reporting a vulnerability

Contact the maintainer privately at **soft-arch@ots.annauniv.edu**. Include the affected commit, operating system, a minimal reproduction using synthetic data, expected behavior, and the security impact. Do not include credentials or private source code, and do not publish exploit details in a public issue before coordination.

Only the current development branch is maintained; there are no supported release versions yet. Response and fix timelines depend on maintainer availability. Avoid running this development build on actively hostile, concurrently modified directories or attacker-controlled caches.
