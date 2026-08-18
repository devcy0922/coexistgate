# Demo scenarios

Each scenario is a pair of trees: `base/` (previous release) and `head/` (candidate).

```bash
cargo run -p coexistgate-cli -- analyze \
  --base-dir demo/scenarios/01-unsafe-db/base \
  --head-dir demo/scenarios/01-unsafe-db/head
```

| Dir | Story | Gate |
| --- | --- | --- |
| 01-unsafe-db | Looks like a clean rename to `email_address`; previous app still reads `users.email`; RollingUpdate | FAIL |
| 02-safe-db | Expand: add `email_address`, keep `email` | PASS |
| 03-availability | replicas 3 → 1 vs `minimum_replicas: 2` | FAIL |
| 04-config-rollback | `REDIS_URL` → `CACHE_URL`; previous app still needs REDIS_URL | FAIL |

These are the same fixtures used in CLI integration tests.
