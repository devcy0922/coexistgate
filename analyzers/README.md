# Analyzers

V0.1 analyzers are Rust modules inside `coexistgate-core` (not a plugin ABI):

| Analyzer | Module |
| --- | --- |
| Migration | `crates/coexistgate-core/src/analyzers/migration.rs` |
| Application | `crates/coexistgate-core/src/analyzers/application.rs` |
| Deployment | `crates/coexistgate-core/src/analyzers/deployment.rs` |
| Configuration | `crates/coexistgate-core/src/analyzers/configuration.rs` |

They emit Facts only. Rules live in `crates/coexistgate-core/src/rules.rs`.
