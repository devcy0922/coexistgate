# ADR 004 — AWS deployment

## Status

Accepted for V0.1.

## Context

GOAL.md: pick one of Terraform, SAM, CloudFormation, Lambda container image — complete, not four half-solutions.

## Decision

- **Terraform** in `deploy/aws` (`terraform init && terraform apply`).
- Runtime: **Lambda container image** (provided.al2023-compatible Dockerfile) so the same musl/static binary story as CLI releases applies without a custom AL2 compiler on the laptop.
- Front door: **API Gateway HTTP API** (payload cap documented; default 256 KB is too small for GitHub webhooks — request `max` 1 MB).
- Secrets: AWS Secrets Manager (webhook secret, GitHub App ID, installation optional, PEM private key).
- IAM: Lambda role can read those secrets, write CloudWatch logs, pull the image. No S3 dump of repo zips.
- Alternative SAM/CloudFormation templates are **not** shipped in V0.1 (`docs/roadmap.md`).

## Consequences

- Users need Docker + Terraform + AWS credentials.
- Cold start is acceptable for PR webhooks; timeout 60s; memory 512 MB default.
