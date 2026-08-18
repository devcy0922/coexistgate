# CoexistGate AWS (Terraform)

Deploys GitHub webhook → API Gateway HTTP API → Lambda in **your** AWS account.

Repository contents are fetched from GitHub into Lambda memory and never sent to a CoexistGate-operated server.

## Prerequisites

- Terraform >= 1.5
- AWS credentials with rights to API Gateway, Lambda, ECR, IAM, Secrets Manager
- Docker (image build)
- A GitHub App (see below)

## GitHub App

Create a GitHub App (user or org):

| Permission | Access |
| --- | --- |
| Checks | Read and write |
| Contents | Read-only |
| Pull requests | Read-only |
| Metadata | Read-only |

Subscribe to **Pull request** events.

After `terraform apply`, set the App webhook URL to `webhook_url` output and the webhook secret to the same value stored in Secrets Manager.

Install the App on the repositories you want scanned.

## Apply

```bash
cd deploy/aws
cp terraform.tfvars.example terraform.tfvars
# edit tfvars

terraform init
terraform apply
```

Outputs:

- `webhook_url` — GitHub App webhook URL
- `secret_arn` — Secrets Manager ARN (`webhook_secret`, `app_id`, `private_key`)
- `lambda_name`

Put the GitHub App private key PEM into the secret (Terraform can seed it from `github_app_private_key_path`).

## GitHub settings checklist

1. App webhook URL = `webhook_url`
2. Webhook secret = the generated (or provided) secret
3. App installed on target repos
4. Permissions as above

## Security notes (V0.1)

- HMAC SHA-256 webhook verification
- Least-privilege IAM (secrets read, logs, ECR pull)
- Request body cap 1 MiB
- Zip-slip rejection; skipped `node_modules` / `target`
- Source file contents are **not** written to CloudWatch
- Timeout 60s, memory 512 MB
- No shell-out in Lambda (GitHub HTTP + in-process core)
