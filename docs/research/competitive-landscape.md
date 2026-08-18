# Competitive landscape (V0.1)

Primary sources reviewed 2026-08-19.

## GitHub Checks

- Check Runs are created via REST `POST /repos/{owner}/{repo}/check-runs`.
- Creating Checks in the full Check API sense requires a **GitHub App** (not a random PAT).
- A GitHub **Actions job** also appears as a check on the PR; that path needs no App.
- Check output: title, markdown summary, annotations (`path`, `start_line`, `annotation_level`, `message`), max 50 annotations per request.
- V0.1 uses both: Actions for the easiest OSS install, GitHub App + webhook for AWS self-host.

## GitHub Webhooks

- `pull_request` events (opened, synchronize, reopened, ready_for_review).
- Verify `X-Hub-Signature-256` = `sha256=` + HMAC-SHA256(raw body, secret). Compare in constant time.
- Do not log payload file contents.

## Copilot Code Review

- Reviews PR diffs; comments on bugs, style, some security; **advisory** (Comment, not blocking approve/request-changes).
- 2026 agentic architecture gathers extra repo context, still **non-deterministic**, not a release-lifecycle proof.
- Does not emit a stable evidence contract linking migration + previous app + RollingUpdate + rollback invariant.
- **Test A:** a Copilot comment “renaming this column might break callers” is not the same product. CoexistGate only fires with connected facts.

## Atlas migration lint

- [Migration analyzers](https://atlasgo.io/lint/analyzers): `incompatible` flags rename/drop of tables/columns because rolling deploys keep old clients.
- Atlas does **not** parse the previous application to see whether `users.email` is still referenced.
- Atlas does **not** read Kubernetes `strategy: RollingUpdate` vs `Recreate`.
- **Test B:** do not clone Atlas BC101–BC104 as standalone SQL lint. CoexistGate uses migration **facts** and only **finds** when application + release strategy (or rollback model) connect.

## Checkov

- IaC/SCA misconfig scanner (Terraform, K8s, Dockerfile, CI, …).
- Replica count vs an **application release policy** in `.coexistgate.yml` is not Checkov’s job.
- **Test B:** do not add CKV-style “Deployment should not run as root”. Only release invariants (replicas vs policy, resource regression vs policy/previous manifest).

## Hadolint

- Dockerfile linter. Out of scope.

## Liquibase

- Changelog/database policy checks, optional rollback-script checks.
- Does not join application source + deploy strategy + env contract.

## Nearby “release safety” tools

- Helixar ReleaseGuard: **build artifact** scan/SBOM/sign. Different layer (`dist/`).
- PyPI releaseguard: mobile debug flags / version bumps.
- SafeToShip / Ship Safe: AI-app launch/security agents.
- Shinka / Atlas Operator: **run** migrations on Kubernetes, not PR-time cross-artifact gating.

## Gap CoexistGate owns

```text
previous application facts
+ candidate schema / config / deploy facts
+ explicit release policy
+ rolling vs rollback lifecycle
→ deterministic, evidence-backed gate
```

No hosted SaaS in V0.1. Analysis stays in CLI, Actions, or the user’s Lambda.
