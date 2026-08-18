# ADR 003 — GitHub event boundary

## Status

Accepted for V0.1.

## Context

GitHub is the first integration. Core must stay host-agnostic. Checks API is App-oriented; Actions jobs already show as checks.

## Decision

- Core input is `AnalysisRequest` only.
- `coexistgate-github` maps webhook/PR metadata → trees + request, and maps `Report` → Check Run output.
- **Path A (default OSS):** GitHub Actions runs the CLI with `actions/checkout` (fetch-depth 0) and `--base` the PR base ref. The job name is `CoexistGate / Release Safety`.
- **Path B (AWS self-host):** GitHub App webhook → user Lambda. App permissions: `checks:write`, `contents:read`, `pull_requests:read`. Installation token creates the Check Run.
- GitLab/Bitbucket adapters are not implemented. The `AnalysisRequest` shape is the extension point.

## Consequences

- Users never send git blobs to a CoexistGate-operated server.
- Path B requires a GitHub App private key in the user’s Secrets Manager — documented, not hidden.
