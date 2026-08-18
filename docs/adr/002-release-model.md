# ADR 002 — Release model

## Status

Accepted for V0.1.

## Context

Rules need a shared meaning of “previous release”, “candidate”, “rolling”, and “rollback” or findings become slogans.

## Decision

- **Previous** = file tree at `--base` (default: `origin/main`, `origin/master`, `main`, `master` if present).
- **Candidate** = working tree / `--head` / PR head SHA.
- **Rolling coexistence** = previous **application** + candidate **schema**, while deploy strategy is rolling (manifest `RollingUpdate` / Compose rolling replica replacement, or policy `release.strategy: rolling` when the manifest has no strategy).
- **Rollback** = previous **application** + candidate **schema** + candidate **configuration/deploy env contract**. Migrations and env contracts are assumed **not** reverted with the app.
- **Recreate** strategy: no mixed-version window, so rolling-coexistence DB rules do not fire; **rollback** DB/config rules still can.
- **Resources / availability**: only explicit policy and/or previous **manifest** facts. No live metrics.

## Consequences

- Matches Demo 1–3 in GOAL.md.
- Kubernetes pod env is replica-set-scoped; rolling config-env coexistence is not treated as sharing one env blob. Shared `.env` files are.
