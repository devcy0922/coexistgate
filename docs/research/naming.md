# Naming research (V0.1)

Date: 2026-08-19

## ReleaseGuard — rejected

| Surface | Collision |
| --- | --- |
| GitHub | [Helixar-AI/ReleaseGuard](https://github.com/Helixar-AI/ReleaseGuard) — artifact policy engine, CLI `releaseguard`, `.releaseguard.yml` |
| Marketplace | “Artifact Policy Engine” GitHub Action |
| PyPI | `releaseguard` — mobile/app pre-production safety gate |
| PyPI/npm | `releaseguard-cli` — PII scan/redact (binary name `releaseguard`) |
| GitHub | Debasish-87/ReleaseGuard — GO/HOLD/NO-GO governance |

CLI binary and policy filename collision is a hard no.

## Alternatives

### 1. CoexistGate (selected)

- Meaning: mixed-version **coexistence** during rolling deploys, plus a merge **gate**
- CLI: `coexistgate` (no known binary collision)
- GitHub user `coexistgate`: not found (2026-08-19)
- npm `coexistgate`: not found
- PyPI `coexistgate`: 404
- Domain: `coexistgate.dev` / `coexistgate.io` did not resolve

Risk: “-gate” can read as a scandal suffix. In this product it is the CI gate metaphor (Release Gate). Acceptable.

### 2. ShipCompat

Clear “shipping compatibility”. Slightly generic; nearby names include SafeToShip, Ship Safe, CloudShip `ship`.

### 3. RelSafe

Good meaning, short CLI. **Rejected:** GitHub org [`RELSAFE`](https://github.com/RELSAFE) and site relsafe.co.in already exist.

## Trademark

No full legal search was performed. No USPTO filing was made. V0.1 ships as CoexistGate; revisit if a real mark conflict appears.

## Config filename

`.coexistgate.yml` — not `.releaseguard.yml`.
