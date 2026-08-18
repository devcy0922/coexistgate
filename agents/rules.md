# CoexistGate — Agent Rules

This file is the project-local override for agent work. It inherits global environment rules and **wins on conflict**.

## Environment

- **Project server:** macmini (`192.168.0.5`) — this workspace
- **LLM gateway:** vLLM on dgx-spark (`192.168.0.7`) via LiteLLM on macmini only if an agent feature needs it
- **V0.1 product rule:** Release Gate is **deterministic**. Do **not** call an LLM to judge PASS/FAIL.

## SSOT

1. Read `GOAL.md` before architecture, features, bug fixes, dependencies, GitHub, AWS, or demos.
2. If implementation and `GOAL.md` conflict, **GOAL.md wins**.
3. Ambiguous choice → pick the option closer to `GOAL.md`, not “keep current code”.

## Product (one sentence)

CoexistGate is a **cross-artifact release safety engine**. It answers: **can this change be safely released and rolled back?** It does not review code quality.

## V0.1 identity checks (every change)

- Finding must be **cross-artifact** and/or **release-lifecycle** when it claims to be a ReleaseGuard-class result.
- Every **blocking** finding needs **evidence** (artifact + location + fact).
- Same `(base, head, policy)` → same findings and gate.
- Core must not import GitHub, Lambda, HTTP, or AWS types.
- Do not reimplement Atlas / Checkov / Hadolint / secret scanners / AI PR review.

## Working loop

1. Read `GOAL.md`
2. State how the task relates to the product goal
3. Confirm it is in V0.1 scope (`docs/roadmap.md` for later ideas)
4. Implement
5. Verify against acceptance gates G1–G18 in `GOAL.md`

## Naming

Public product name is **CoexistGate** (CLI `coexistgate`). Do not ship a `releaseguard` binary. See `docs/research/naming.md`.

## Stop conditions

If a change would make core GitHub/AWS-dependent, make the gate LLM-judged, or require heavy user annotation to get signal, stop and report. Do not “just add a feature”.
