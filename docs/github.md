# GitHub integration

Two supported paths. Core is unchanged.

## A. GitHub Actions (simplest)

Copy [`examples/github-action.yml`](../examples/github-action.yml). The job name is the check **CoexistGate / Release Safety**. Uses `actions/checkout` with `fetch-depth: 0` and `--base origin/<base_ref>`.

No GitHub App. Contents stay on GitHub-hosted (or your) runners.

## B. Webhook + AWS Lambda

GitHub App → API Gateway → Lambda in the **user** account. See [`deploy/aws/README.md`](../deploy/aws/README.md).

Lambda verifies `X-Hub-Signature-256`, fetches base/head zipballs, runs core, posts a Check Run.

Required App permissions: Checks (write), Contents (read), Pull requests (read). Events: `pull_request`.
