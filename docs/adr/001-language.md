# ADR 001 — Language and application parser

## Status

Accepted for V0.1.

## Context

GOAL.md prefers Rust for CLI distribution unless parsers or velocity block V0.1. Application language may be limited to one stack. AST vs regex is an explicit decision.

## Decision

- **Implementation language: Rust** (edition 2021). Toolchain is present; `cargo-zigbuild` can produce Linux musl binaries; Lambda can run a static binary; no LLM in the gate.
- **Go** would also work; it was not required. TypeScript for core would weaken single-binary + Lambda cold-start story.
- **Application analyzer language: TypeScript/JavaScript only.** Killer demo references (`users.email`, `process.env.REDIS_URL`) live here; one language keeps facts deterministic.
- **Parser: comment-aware token/regex, not a TS AST.** Qualified identifiers, `process.env`, and SQL-in-string `FROM` clauses are regular enough. Shipping `swc`/`tree-sitter` into Lambda increases size and failure modes without proving extra true positives for V0.1.

## Consequences

- Python/Go/Java application references are false negatives (documented limitation).
- Ambiguous `row.email` without a table qualifier is not claimed as `users.email` (no guessing — STOP C).
- If STOP B hits in real repos, revisit AST in a later version — not by adding an LLM judge.
