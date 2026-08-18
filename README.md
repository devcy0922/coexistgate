# CoexistGate

**Release safety checks your code review doesn't cover.**

> Can this change be safely released and rolled back?

Your code passed. Your tests passed. Your rollback didn't.

```text
✓ Build
✓ Tests
✓ Code Review
✗ CoexistGate / Release Safety
```

CoexistGate is a **cross-artifact release safety engine**. It does not review whether code is “correct”. It asks whether **migration + previous application + deployment + policy** can coexist in production and whether the previous version can still be restored.

```text
migration.sql  +  src/*.ts  +  deployment.yaml  +  .coexistgate.yml
                         ↓
                  Release Gate
```

## Visual proof

```text
GitHub PR
    ├─ Build .............. PASS
    ├─ Tests .............. PASS
    └─ CoexistGate ........ FAIL
           │
           ├─ migrations/002.sql:3   RENAME COLUMN email → email_address
           ├─ src/user_repository.ts:48   previous release: users.email
           └─ deploy/deployment.yaml:17   RollingUpdate
                    ↓
           Compatibility FAIL    Rollback UNSAFE
```

Real scenario trees (run locally without GitHub):

| Scenario | Expected gate |
| --- | --- |
| [demo/scenarios/01-unsafe-db](demo/scenarios/01-unsafe-db) | FAIL |
| [demo/scenarios/02-safe-db](demo/scenarios/02-safe-db) | PASS |
| [demo/scenarios/03-availability](demo/scenarios/03-availability) | FAIL |
| [demo/scenarios/04-config-rollback](demo/scenarios/04-config-rollback) | FAIL |

Public demo PRs (created at release time) are linked from the [project site](https://devcy0922.github.io/coexistgate/).

## What it catches

- Database compatibility across mixed versions
- Rollback compatibility (old app + new schema/config)
- Deployment availability vs explicit policy
- Configuration contract drift
- Cross-artifact release risks with **evidence** (artifact, line, fact)

It is **not** Copilot, Atlas, Checkov, Hadolint, or a secret scanner.

## Run locally

```bash
# from source
cargo install --path crates/coexistgate-cli

coexistgate analyze .
coexistgate analyze --base origin/main
coexistgate analyze --format json
coexistgate analyze --base origin/main --fail-on high
coexistgate rules
coexistgate explain DB-BACKWARD-COMPAT-001
```

Fixture (no git history required):

```bash
coexistgate analyze \
  --base-dir demo/scenarios/01-unsafe-db/base \
  --head-dir demo/scenarios/01-unsafe-db/head
```

### Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Analysis finished; gate **PASS** |
| 1 | Analysis finished; gate **FAIL** |
| 2 | Usage / I/O / invalid policy |

`--fail-on` overrides `gate.fail_on` in `.coexistgate.yml` for that run.

## GitHub

Add the workflow in [`examples/github-action.yml`](examples/github-action.yml). The job appears as **CoexistGate / Release Safety**.

Self-host the same engine on a GitHub App webhook: [`deploy/aws`](deploy/aws).

## AWS self-host

```bash
cd deploy/aws
cp terraform.tfvars.example terraform.tfvars
terraform init && terraform apply
```

Outputs include webhook URL, required GitHub App settings, and secret names. Private repository contents stay in **your** AWS account. See [`deploy/aws/README.md`](deploy/aws/README.md).

## How it differs

**Code review:** Is this code correct?

**CoexistGate:** Can these changes safely coexist in production, and can the previous version still be restored?

Copilot comments on a diff. Atlas lints a migration in isolation. Checkov lints IaC security. CoexistGate **connects** previous application facts, candidate schema/config/deploy facts, and an explicit release policy, then emits a **deterministic** gate with evidence.

## Architecture

```text
Event  →  Adapter  →  AnalysisRequest  →  Core  →  Report
                         ↑
              Analyzers emit Facts only
```

- **Core** (`coexistgate-core`): discovery, facts, release model, rules, gate
- **CLI** (`coexistgate`): local / CI
- **GitHub adapter**: webhook HMAC, zipball trees, Check Runs
- **Lambda**: user-account webhook process

Core does not import GitHub, AWS, or HTTP types. The release gate does not call an LLM.

## Supported in V0.1

| Area | Support |
| --- | --- |
| Application | TypeScript / JavaScript references (`table.column`, `process.env.KEY`) |
| Migrations | PostgreSQL DDL facts (rename/drop/type/NOT NULL) |
| Deployment | Kubernetes `Deployment`, Docker Compose |
| Configuration | `.env` / `.env.example` and deploy env |
| Policy | `.coexistgate.yml` |
| Languages (app) | **Not** Python/Go/Java |
| Databases | **Not** MySQL/Prisma migrate (facts only for Postgres SQL files) |

Copy [`.coexistgate.yml.example`](.coexistgate.yml.example) to `.coexistgate.yml`.

## Limitations

- Application facts are regex/token based, not a TypeScript AST. Unqualified `row.email` is **not** claimed as `users.email`.
- Helm templates are skipped.
- No live cluster metrics; resource/replica rules use policy and manifests only.
- Rollback is modeled as **previous application + candidate schema + candidate config contract**.
- Package registries are not published in V0.1 (GitHub Releases + source build).
- The working name “ReleaseGuard” was **not** used: it collides with an existing `releaseguard` CLI. See [`docs/research/naming.md`](docs/research/naming.md).

## Install from a GitHub Release

See [`docs/install.md`](docs/install.md). Verify checksums; do not pipe unknown scripts to a shell without reading them.

## License

Apache-2.0. See [`GOAL.md`](GOAL.md) for the product SSOT.
