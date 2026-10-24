# Security Policy

## Supported versions

CodeAtlas is alpha software. Only the latest release and the `main` branch
receive fixes.

## Reporting a vulnerability

Please do not open a public issue for a security vulnerability. Use GitHub's
[private vulnerability reporting](https://github.com/Mullassery/CodeAtlas/security/advisories/new)
for this repository. If that is unavailable, open an issue titled
"Security contact request" with no vulnerability details.

There is no guaranteed response time or SLA.

## Security model

- **Static analysis only.** CodeAtlas reads repository files; it does not run
  the code it analyzes.
- **LLM use is opt-in.** When a provider is configured, component names and
  descriptions may be sent to that provider. Without a provider, analysis stays
  local and deterministic.
- **Treat analyzed repositories as untrusted input.** Path traversal, malformed
  source, and oversized files are handled as parse failures, not executed.
- **Generated HTML is a report.** It is not a sandbox for untrusted content; do
  not host it on an origin with privileged cookies.
- **Dependency scanning runs in CI.** No audit proves the absence of
  vulnerabilities.

## Scope

The Rust crates, Python CLI, renderers, and repository workflows are in scope.
Third-party LLM providers and your CI environment are out of scope.
