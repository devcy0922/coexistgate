# GOAL.md — CoexistGate SSOT

This document is the single source of truth for CoexistGate V0.1.

If architecture, code, docs, demos, or marketing disagree with this file, **this file wins**.

When a choice is ambiguous, do not ask “should we keep the current implementation?” Ask: **which option is closer to this Goal?**

---

## Product Goal

CoexistGate is a **Cross-artifact Release Safety Engine**.

Core question:

> **Can this change be safely released and rolled back?**

CoexistGate does not evaluate whether code is “good”.

It analyzes whether several changes, when they exist together in a real production rollout:

- keep compatibility
- allow a rolling deployment
- allow rollback
- preserve availability invariants
- preserve configuration contracts

## Core Product Principle

Not individual artifact correctness.

> **Inspect relationships between artifacts and the release lifecycle.**

```text
migration.sql
+
application code
+
deployment manifest
+
release policy
        ↓
Release Safety
```

Each file can look fine in isolation. The combination can still be unsafe. That combination is the product.

## CoexistGate is not

- AI Code Reviewer / GitHub Copilot replacement / LLM PR Reviewer
- SAST, secret scanner, or general security scanner
- Checkov / Hadolint / Atlas / SQL linter / Dockerfile linter / Kubernetes linter replacement
- CI/CD orchestrator, deployment platform, workflow engine, or AI agent framework

Do not reimplement specialist single-artifact lint.

## Primary Differentiator

A finding should be:

```text
cross-artifact + deterministic + evidence-backed + release-aware + rollback-aware
```

Low value:

```text
"This setting might be risky."
```

Target:

```text
Release Gate: BLOCK

Reason:
migration.sql:13 removes users.email

Previous release:
src/user_repository.ts:48 references users.email

Deployment:
RollingUpdate allows previous and new versions
to coexist.

Result:
Previous application version is incompatible
with the candidate schema.

Rollback:
UNSAFE
```

## OSS Goal

Users must not upload repository contents to a CoexistGate-operated SaaS.

Supported directions:

```text
Local CLI
GitHub Integration (Actions and/or self-hosted webhook)
AWS Lambda (user account)
```

Analysis runs in the user's execution environment.

## V0.1 Success

Not a toy demo.

An external developer must be able to:

```bash
coexistgate analyze .
```

or connect their own GitHub PR, and see real findings in distinct release-risk categories.

Definition of Done (public repo, first visit):

1. Understand the product in ~20 seconds
2. Open a real failing PR
3. Open a real safe PR
4. Install the CLI
5. Analyze a local repo
6. Connect a GitHub PR
7. Self-host on their AWS account

Results must look like:

> These three artifacts' three facts break this release invariant.

Not:

> AI thought this was risky.

---

## Working name decision

Proposed name **ReleaseGuard** is **rejected for V0.1 shipping**.

Severe collisions:

| Collision | Why it is severe |
| --- | --- |
| Helixar-AI/ReleaseGuard | Same product-shaped name, CLI binary `releaseguard`, config `.releaseguard.yml`, GitHub Marketplace action |
| PyPI `releaseguard` | Pre-production release safety gate CLI |
| PyPI/npm `releaseguard-cli` | PII redaction tool using the `releaseguard` binary |
| Debasish-87/ReleaseGuard | Release GO/NO-GO governance |

Shipping another `releaseguard` binary would install over / confuse an existing OSS tool.

### Alternatives considered

| Name | Meaning | Search | Dev-tool feel | Registry/domain notes |
| --- | --- | --- | --- | --- |
| **CoexistGate** | Mixed-version coexistence during rolling + a release **gate** | Distinct | Strong | GitHub user free; npm/pypi free; `coexistgate.dev` did not resolve at research time |
| ShipCompat | Shipping compatibility | Somewhat generic | Strong | GitHub user free; npm/pypi free |
| RelSafe | Release safety | Weak (RELSAFE org + relsafe.co.in exists) | Strong | GitHub org `RELSAFE` already exists |

**Shipped name: CoexistGate**

- Tagline: **Release safety checks your code review doesn't cover.**
- Support: Your code passed. Your tests passed. Your rollback didn't.
- CLI: `coexistgate`
- GitHub Check: `CoexistGate / Release Safety`
- Repository: `coexistgate`
- Policy file: `.coexistgate.yml`

Internal workspace directory may remain `release-guard`. Public identity is CoexistGate.

---

## V0.1 functional scope

Four analyzers, real, not stubs:

1. Application Analyzer
2. Migration Analyzer
3. Deployment Analyzer
4. Configuration Analyzer

Core must not switch on analyzer kinds. Analyzers register, emit Facts, and stop.

### Analyzer 1 — Migration

- PostgreSQL DDL facts first
- Extract operations: `DROP COLUMN`, `RENAME COLUMN`, incompatible type change, `NOT NULL` introduction, destructive constraint change, table/column rename
- Purpose is **facts**, not a general migration linter (not Atlas)

### Analyzer 2 — Application

- V0.1 language: **TypeScript / JavaScript only**
- Why: the killer demo is `users.email` + `process.env.REDIS_URL` in the same class of service repos that use Kubernetes/Compose; one language keeps the gate deterministic
- Method: **comment-aware token/regex extraction**, not a full TS AST (see ADR 001). Qualified `table.column`, SQL-in-string `FROM`, and env access are enough for V0.1 accuracy. Full AST is a roadmap item if false negatives become a STOP-B issue.

### Analyzer 3 — Deployment

- Kubernetes `Deployment` and Docker Compose
- Facts only: strategy, replicas, resource limits, environment variables
- Not a Kubernetes linter

### Analyzer 4 — Configuration

- `.env` / `.env.example` style key/value and deployment env definitions
- Connect rename/removal to previous-release dependency and runtime contract
- **Not** secret scanning

## Rule target

- About **15–25** built-in rules
- No padding rules
- **At least 5** are true cross-artifact rules

Required families:

| ID | Family |
| --- | --- |
| DB-BACKWARD-COMPAT-001 | Schema candidate + previous application + rolling/coexistence |
| DB-ROLLBACK-COMPAT-001 | Candidate schema + previous application (rollback = old app, new schema) |
| CONFIG-ROLLBACK-COMPAT-001 | Previous app env vs candidate config/deploy contract |
| DEPLOY-REPLICA-001 | Replicas vs `.coexistgate.yml` availability policy |
| DEPLOY-RESOURCE-001 | Candidate resources vs **explicit** policy or previous manifest (no runtime metrics, no guessing) |

## Rollback model (V0.1)

**Rollback** means:

> Restore the **previous application version** while **candidate schema** and **candidate configuration contract** remain.

This matches the usual production failure: migrations are not auto-reverted; shared config or already-applied env may remain; the app binary/ReplicaSet is rolled back.

**Rolling coexistence** means:

> Previous and candidate application versions run at the same time against **candidate schema**.

Kubernetes pod-spec env is versioned with the ReplicaSet, so old pods keep old env during rolling. Shared `.env` / mutated config files are not.

## Release policy

Repository root `.coexistgate.yml`. Keep the DSL small. See `.coexistgate.yml.example`.

## Core architecture

```text
Inputs → Artifact Discovery → Facts → Release Model
      → Cross-Artifact Rules → Findings → Release Gate → Report
```

Core owns: artifact/fact/release models, rule engine, finding, evidence, severity, policy, gate, report, analyzer registry.

Core must not know: GitHub webhooks, AWS Lambda, API Gateway, website, GitHub tokens, HTTP frameworks.

Event boundary:

```text
Event → Adapter → AnalysisRequest → Core → Report
```

V0.1 implements GitHub. GitLab/Bitbucket are out of scope.

## Finding contract

Minimum fields: `rule_id`, `severity`, `category`, `title`, `evidence[]`, `impact`, `release_impact`, `rollback_impact`, optional `recommendation`.

Evidence: `artifact`, `location`, `fact`.

Every blocking finding has evidence. No evidence-free severity.

## LLM boundary

V0.1 gate does **not** use an LLM. PASS/FAIL is deterministic.

Forbidden: diff → LLM → risk judgment.

Allowed later: finding → optional LLM → explanation. LLM may be Explainer, never Judge.

## CLI

```bash
coexistgate analyze .
coexistgate analyze --base origin/main
coexistgate analyze --format json
coexistgate rules
coexistgate explain DB-BACKWARD-COMPAT-001
coexistgate analyze --base origin/main --fail-on high
```

### Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Analysis completed; gate PASS (no finding at or above `--fail-on` / policy `gate.fail_on`) |
| 1 | Analysis completed; gate FAIL |
| 2 | Usage error, I/O error, or invalid policy |

`--fail-on` on the CLI overrides policy `gate.fail_on` for that run.

## GitHub

First event integration. Adapter verifies webhook signatures, loads PR base/head, builds `AnalysisRequest`, posts Check Run `CoexistGate / Release Safety`.

Also supported: GitHub Actions running the CLI so the workflow job itself is the check (no CoexistGate SaaS).

## AWS

One complete self-host path: **Terraform** → API Gateway HTTP API → Lambda (container image or provided.al2023 binary) → GitHub API.

Private repository contents stay in the **user's** AWS account. Outputs: webhook URL, GitHub App settings, secret names.

## Demo

Proof is a **healthy-looking PR that CoexistGate blocks for release risk**, plus a safe counterpart.

Minimum:

1. Unsafe DB migration (rolling + previous `users.email`) → FAIL
2. Expand/contract safe DB migration → PASS or REVIEW
3. Availability regression (`replicas: 3` → `1` vs `minimum_replicas: 2`) → FAIL
4. Config rollback break (`REDIS_URL` → `CACHE_URL`) → FAIL

Public GitHub PRs beat a marketing video. README gets a lightweight visual; website may use WebM later.

## Website

Separate public site whose only job is: a developer understands the differentiator in ~20 seconds.

No SaaS signup, no upload, no fake metrics, no “AI-powered” claims, only features that exist.

## Distribution

GitHub Release: Linux and macOS binaries. Docker image if practical. Package registries are not required for V0.1.

## Testing

Unit, parser, rule, policy, CLI integration, unsafe/safe fixtures, webhook signature, GitHub adapter, Lambda adapter, deterministic output. Golden fixtures stay in git.

## Determinism

Same base + head + policy → same findings and gate. No clocks or LLM output in the decision.

## Observability

No dashboard. Structured logs: request id, analyzer durations, rule counts, gate. Never log source contents or secrets.

## Performance

No strict benchmark. Prefer changed artifacts + required previous-tree dependencies. Skip `node_modules`, `target`, `.git`, and oversized files.

## GOAL-based development

Each phase:

1. Read this file
2. State the product-goal relation
3. Confirm in-scope
4. Implement
5. Verify the result against this Goal

Bugs: name the invariant that broke, then fix it.

## Scope guard — do not add in V0.1

Terraform **scanning**, full Kubernetes/Nginx/backup/runbook/MCP scanners, AI review, LLM explanation, dashboard, auth, database, billing, orgs, Slack, GitLab, Bitbucket, scheduled scanning, hosted SaaS.

Record later ideas in `docs/roadmap.md` only.

## Acceptance gates

| Gate | Requirement |
| --- | --- |
| G1 GOAL | This file exists; architecture follows it |
| G2 Core | Core analyzes fixtures without GitHub/Lambda |
| G3 Analyzers | Application, Migration, Deployment, Configuration exist |
| G4 Cross-artifact | ≥ 5 real cross-artifact rules |
| G5 Rule set | ~15–25 meaningful rules |
| G6 CLI | `coexistgate analyze .` works on a real repo |
| G7 Git diff | base/head comparison works |
| G8 Evidence | Blocking findings have rule, artifact, location, fact, impact |
| G9 Deterministic | Same input → same output |
| G10 GitHub | Real PR Check runs |
| G11 Demo 1 | Unsafe migration blocked |
| G12 Safe fix | Safe migration PASS/REVIEW |
| G13 Demo 2 | Availability regression detected |
| G14 Demo 3 | Config rollback break detected |
| G15 AWS | Documented Lambda deploy is reproducible |
| G16 Data boundary | Full product without uploading to CoexistGate SaaS |
| G17 Website | Public demo site |
| G18 OSS release | Binary or documented install |

## STOP conditions (halt feature expansion and report)

| ID | Condition |
| --- | --- |
| STOP A | Core cross-artifact findings cannot be decided reliably |
| STOP B | Application references cannot be connected stably |
| STOP C | Most results become heuristic/LLM guesses |
| STOP D | Differentiator vs Copilot/Atlas/Checkov disappears |
| STOP E | Meaningful results require heavy user annotation |
| STOP F | Core starts depending on a specific GitHub/AWS environment |

## Competitive tests (every major finding)

- **A.** Could Copilot Code Review give the same result from ordinary review? If yes, low value.
- **B.** Does Checkov/Hadolint/Atlas already solve this exactly? If yes, do not reimplement.
- **C.** Is this only findable by linking artifacts/release states? Prefer yes.
