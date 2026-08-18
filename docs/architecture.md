# Architecture — CoexistGate V0.1

SSOT: [`GOAL.md`](../GOAL.md). This file describes what is implemented, not a future platform.

## Context

```mermaid
flowchart LR
  subgraph userEnv [User environment]
    CLI[coexistgate CLI]
    Actions[GitHub Actions]
    Lambda[AWS Lambda in user account]
  end
  subgraph coreBox [coexistgate-core]
    Disc[Artifact discovery]
    Facts[Facts]
    Model[Release model]
    Rules[Rule engine]
    Gate[Release gate]
  end
  Repo[Git repository]
  GH[GitHub API]
  CLI --> Disc
  Actions --> CLI
  Lambda --> GH
  Lambda --> Disc
  Repo --> CLI
  Disc --> Facts --> Model --> Rules --> Gate
```

Core never imports GitHub, Lambda, or HTTP types.

## Pipeline

```text
Inputs
  → Artifact Discovery
  → Facts
  → Release Model
  → Cross-Artifact Rules
  → Findings
  → Release Gate
  → Report
```

### Inputs

`AnalysisRequest`:

- `previous`: file tree at base (previous release)
- `candidate`: file tree at head / working copy
- `policy`: parsed `.coexistgate.yml` (candidate, fallback previous)
- `changed_paths`: optional hint; discovery still may load previous app/config needed by rules

Trees are `BTreeMap<path, bytes>` so iteration is deterministic.

### Discovery

Classifies files by path/extension. Skips `.git`, `node_modules`, `target`, `dist`, `vendor`, files over 1 MiB.

Does not parse the whole world when a change set exists: always load candidate + previous files that analyzers mark as **needed** (application/config when migrations or env files changed; deploy manifests when policy/availability rules run).

### Analyzers

| Analyzer | Emits |
| --- | --- |
| Migration | PostgreSQL DDL schema operations |
| Application | TS/JS `table.column` and env references |
| Deployment | K8s Deployment + Compose strategy, replicas, resources, env |
| Configuration | `.env*` keys |

Analyzers implement:

```text
fn id() -> &'static str
fn analyze(tree, ctx) -> Vec<Fact>
```

Core holds a registry. Adding an analyzer does not change the rule engine.

### Release model

Two snapshots of facts plus policy:

- Previous application references vs candidate schema (rolling + rollback)
- Previous application env vs candidate deploy/config contract (rollback)
- Candidate application env vs candidate deploy (forward)
- Candidate replicas/resources vs policy (and optional no-regression vs previous manifest)

### Rules

Pure functions `ReleaseModel -> Vec<Finding>`. No I/O. Sorted by `(rule_id, first evidence path, line)` before emit.

### Gate

`fail_on` severities (policy or CLI). Finding severity after per-rule overlay (`error` / `warning` / `off`).

`REVIEW` is not a separate gate state in V0.1: unused expand-only schema changes simply produce **no blocking finding** (PASS). Medium findings without `fail_on: medium` also PASS the gate.

## Crate map

```text
coexistgate-core      models, discovery, analyzers, rules, gate, report
coexistgate-cli       git trees, flags, human/JSON output, exit codes
coexistgate-github    webhook HMAC, zipball trees, Check Run formatting
coexistgate-lambda    HTTP entry; calls github + core only
```

Analyzers live in `coexistgate-core` as modules (not a plugin ABI). A second analyzer must not force a core rewrite; it must not require one either.

## Adapters

```text
Event → Adapter → AnalysisRequest → Core → Report
```

GitHub adapter:

1. Verify signature
2. Ignore non-PR or closed PRs
3. Fetch base and head zipballs (not a CoexistGate SaaS cache)
4. Run core
5. Create Check Run `CoexistGate / Release Safety`

Lambda adds: size limits, timeout, no body-content logging, path-zip-slip checks.

## Website

Static HTML in `website/`. No API, no auth. It is not part of core.

## Non-goals in this diagram

No event bus, no org database, no LLM judge, no dashboard.
