use crate::fact::{Fact, LocatedFact, SchemaOp, StrategyKind};
use crate::finding::{Category, Evidence, Finding, Impact, Severity};
use crate::policy::{OverrideLevel, Policy, StrategyHint};
use crate::release_model::{schema_targets_ref, ReleaseModel};
use crate::ENGINE_VERSION;

#[derive(Clone, Debug)]
pub struct RuleMeta {
    pub id: &'static str,
    pub title: &'static str,
    pub category: Category,
    pub default_severity: Severity,
    pub cross_artifact: bool,
    pub explanation: &'static str,
}

pub fn rule_catalog() -> Vec<RuleMeta> {
    catalog().to_vec()
}

fn catalog() -> &'static [RuleMeta] {
    &[
        RuleMeta {
            id: "ANALYZER-COVERAGE-001",
            title: "Analyzer could not establish complete release evidence",
            category: Category::Compatibility,
            default_severity: Severity::High,
            cross_artifact: true,
            explanation: "A supported artifact contains malformed, ambiguous, or unsupported change syntax. The gate fails closed because an empty fact set cannot prove release safety.",
        },
        RuleMeta {
            id: "DB-BACKWARD-COMPAT-001",
            title: "Schema change breaks previous release during rolling coexistence",
            category: Category::Compatibility,
            default_severity: Severity::Critical,
            cross_artifact: true,
            explanation: "Candidate schema removes or renames an object that the previous application still references, and the deployment strategy allows previous and candidate versions to coexist.",
        },
        RuleMeta {
            id: "DB-ROLLBACK-COMPAT-001",
            title: "Schema change makes application rollback unsafe",
            category: Category::Rollback,
            default_severity: Severity::Critical,
            cross_artifact: true,
            explanation: "Rollback restores the previous application while candidate schema remains. Previous application references are incompatible with that schema.",
        },
        RuleMeta {
            id: "DB-NOTNULL-COMPAT-001",
            title: "NOT NULL change is incompatible with previous application",
            category: Category::Compatibility,
            default_severity: Severity::High,
            cross_artifact: true,
            explanation: "SET NOT NULL on a column still used by the previous application can fail writes from mixed versions.",
        },
        RuleMeta {
            id: "DB-ADD-NOTNULL-001",
            title: "NOT NULL column added without DEFAULT",
            category: Category::Compatibility,
            default_severity: Severity::High,
            cross_artifact: true,
            explanation: "Previous application inserts into the table without the new column will fail after the migration.",
        },
        RuleMeta {
            id: "DB-TYPE-COMPAT-001",
            title: "Incompatible column type change vs previous application",
            category: Category::Compatibility,
            default_severity: Severity::High,
            cross_artifact: true,
            explanation: "Column type change plus a previous-application reference is a mixed-version and rollback hazard.",
        },
        RuleMeta {
            id: "DB-CANDIDATE-APP-001",
            title: "Candidate application references a column the candidate schema removes",
            category: Category::Compatibility,
            default_severity: Severity::High,
            cross_artifact: true,
            explanation: "Head application still references table.column that candidate migrations drop or rename.",
        },
        RuleMeta {
            id: "CONFIG-ROLLBACK-COMPAT-001",
            title: "Candidate config contract drops env the previous application requires",
            category: Category::Configuration,
            default_severity: Severity::Critical,
            cross_artifact: true,
            explanation: "Rollback keeps candidate configuration/deploy env. Previous application still reads a key that candidate no longer provides.",
        },
        RuleMeta {
            id: "CONFIG-FORWARD-COMPAT-001",
            title: "Candidate application requires env missing from candidate deploy/config",
            category: Category::Configuration,
            default_severity: Severity::High,
            cross_artifact: true,
            explanation: "New application version references an environment variable that the candidate deployment and dotenv files do not define.",
        },
        RuleMeta {
            id: "CONFIG-SHARED-ROLLING-001",
            title: "Shared dotenv lost a key the previous application needs during rolling",
            category: Category::Configuration,
            default_severity: Severity::High,
            cross_artifact: true,
            explanation: "Unlike pod-spec env, a shared .env file is one contract for all versions. Removing a key breaks previous pods still running.",
        },
        RuleMeta {
            id: "DEPLOY-REPLICA-001",
            title: "Candidate replicas below policy minimum",
            category: Category::Availability,
            default_severity: Severity::High,
            cross_artifact: true,
            explanation: "Deployment replica count is lower than availability.minimum_replicas in .coexistgate.yml.",
        },
        RuleMeta {
            id: "DEPLOY-REPLICA-REGRESSION-001",
            title: "Replica count regressed versus previous release",
            category: Category::Availability,
            default_severity: Severity::High,
            cross_artifact: true,
            explanation: "availability.deny_regression is set and candidate replicas are lower than the previous manifest.",
        },
        RuleMeta {
            id: "DEPLOY-REPLICAS-ZERO-001",
            title: "Deployment replicas set to zero",
            category: Category::Availability,
            default_severity: Severity::Critical,
            cross_artifact: false,
            explanation: "replicas: 0 removes serving capacity. Release-aware availability invariant.",
        },
        RuleMeta {
            id: "DEPLOY-RESOURCE-MEM-001",
            title: "Memory limit below policy minimum",
            category: Category::Resource,
            default_severity: Severity::Medium,
            cross_artifact: true,
            explanation: "Candidate memory limit is below resources.minimum_memory. No runtime metrics are guessed.",
        },
        RuleMeta {
            id: "DEPLOY-RESOURCE-CPU-001",
            title: "CPU limit below policy minimum",
            category: Category::Resource,
            default_severity: Severity::Medium,
            cross_artifact: true,
            explanation: "Candidate CPU limit is below resources.minimum_cpu.",
        },
        RuleMeta {
            id: "DEPLOY-RESOURCE-REGRESSION-001",
            title: "Compute resources regressed versus previous manifest",
            category: Category::Resource,
            default_severity: Severity::Medium,
            cross_artifact: true,
            explanation: "resources.deny_regression is set and candidate limits are lower than previous limits.",
        },
        RuleMeta {
            id: "DEPLOY-RECREATE-DOWNTIME-001",
            title: "Recreate strategy conflicts with availability policy",
            category: Category::Availability,
            default_severity: Severity::Medium,
            cross_artifact: true,
            explanation: "Recreate takes all pods down. When availability.minimum_replicas is set, that is an explicit availability failure during rollout.",
        },
        RuleMeta {
            id: "DEPLOY-MAX-UNAVAILABLE-001",
            title: "maxUnavailable allows all replicas to drop",
            category: Category::Availability,
            default_severity: Severity::High,
            cross_artifact: true,
            explanation: "RollingUpdate maxUnavailable of 100% or equal to replica count violates the availability invariant in policy.",
        },
        RuleMeta {
            id: "DB-EXPAND-CONTRACT-HINT-001",
            title: "Single-step rename without expand column",
            category: Category::Compatibility,
            default_severity: Severity::Critical,
            cross_artifact: true,
            explanation: "A column rename in one migration with previous application still using the old name. Expand (add column) then contract (drop later) is the safe counterpart.",
        },
    ]
}

pub fn evaluate(model: &ReleaseModel) -> Vec<Finding> {
    let mut findings = Vec::new();
    findings.extend(db_backward(model));
    findings.extend(db_rollback(model));
    findings.extend(db_notnull(model));
    findings.extend(db_add_notnull(model));
    findings.extend(db_type(model));
    findings.extend(db_candidate_app(model));
    findings.extend(db_expand_hint(model));
    findings.extend(config_rollback(model));
    findings.extend(config_forward(model));
    findings.extend(config_shared_rolling(model));
    findings.extend(deploy_replica(model));
    findings.extend(deploy_replica_regression(model));
    findings.extend(deploy_replicas_zero(model));
    findings.extend(deploy_mem(model));
    findings.extend(deploy_cpu(model));
    findings.extend(deploy_resource_regression(model));
    findings.extend(deploy_recreate(model));
    findings.extend(deploy_max_unavail(model));
    findings
}

pub fn apply_policy(mut findings: Vec<Finding>, policy: &Policy) -> Vec<Finding> {
    findings.retain_mut(|f| match policy.override_for(&f.rule_id) {
        Some(OverrideLevel::Off) => false,
        Some(OverrideLevel::Warning) => {
            if f.severity > Severity::Medium {
                f.severity = Severity::Medium;
            }
            true
        }
        Some(OverrideLevel::Error) | None => true,
    });
    findings.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    findings
}

fn meta(id: &str) -> &'static RuleMeta {
    catalog().iter().find(|m| m.id == id).expect("known rule")
}

fn ev(f: &LocatedFact, text: impl Into<String>) -> Evidence {
    Evidence {
        artifact: f.path.clone(),
        line: f.line,
        fact: text.into(),
    }
}

fn policy_ev(model: &ReleaseModel, text: impl Into<String>) -> Evidence {
    Evidence {
        artifact: model
            .policy_path
            .clone()
            .unwrap_or_else(|| ".coexistgate.yml".into()),
        line: 1,
        fact: text.into(),
    }
}

fn strategy_evidence(model: &ReleaseModel) -> Evidence {
    if let Some(f) = model.candidate.strategy_fact() {
        let kind = match f.fact {
            Fact::DeploymentStrategy {
                kind: StrategyKind::Rolling,
            } => "RollingUpdate",
            Fact::DeploymentStrategy {
                kind: StrategyKind::Recreate,
            } => "Recreate",
            _ => "strategy",
        };
        return ev(f, format!("Deployment strategy {kind}"));
    }
    let hint = match model.policy.release_strategy {
        Some(StrategyHint::Recreate) => "recreate",
        _ => "rolling",
    };
    policy_ev(model, format!("release.strategy: {hint}"))
}

fn is_rolling(model: &ReleaseModel) -> bool {
    model.candidate.strategy(&model.policy) == StrategyKind::Rolling
}

fn schema_label(f: &LocatedFact) -> String {
    match &f.fact {
        Fact::SchemaChange {
            table,
            operation,
            from,
            to,
            raw,
            ..
        } => format!(
            "{}.{} {} → {} ({})",
            table,
            operation.as_str(),
            from.as_deref().unwrap_or("-"),
            to.as_deref().unwrap_or("-"),
            raw
        ),
        _ => "schema change".into(),
    }
}

fn breaking_schema(model: &ReleaseModel) -> Vec<&LocatedFact> {
    model
        .new_schema_changes()
        .into_iter()
        .filter(|f| match f.fact {
            Fact::SchemaChange { operation, .. } => operation.is_breaking_for_readers(),
            _ => false,
        })
        .collect()
}

fn prev_refs_for<'a>(model: &'a ReleaseModel, change: &'a LocatedFact) -> Vec<&'a LocatedFact> {
    let Fact::SchemaChange {
        table,
        operation,
        from,
        ..
    } = &change.fact
    else {
        return Vec::new();
    };
    match operation {
        SchemaOp::DropTable | SchemaOp::RenameTable => model
            .previous
            .table_refs()
            .into_iter()
            .filter(|(_, t)| *t == table.as_str())
            .map(|(f, _)| f)
            .collect(),
        SchemaOp::DropColumn
        | SchemaOp::RenameColumn
        | SchemaOp::TypeChange
        | SchemaOp::SetNotNull => {
            let col = from.as_deref().unwrap_or("");
            model
                .previous
                .column_refs()
                .into_iter()
                .filter(|(_, t, c)| {
                    schema_targets_ref(&change.fact, t, Some(c)) || (*t == table && *c == col)
                })
                .map(|(f, _, _)| f)
                .collect()
        }
        SchemaOp::AddNotNullColumn => model
            .previous
            .table_refs()
            .into_iter()
            .filter(|(_, t)| *t == table.as_str())
            .map(|(f, _)| f)
            .collect(),
    }
}

fn cand_refs_for<'a>(model: &'a ReleaseModel, change: &'a LocatedFact) -> Vec<&'a LocatedFact> {
    let Fact::SchemaChange {
        table,
        operation,
        from,
        ..
    } = &change.fact
    else {
        return Vec::new();
    };
    match operation {
        SchemaOp::DropTable | SchemaOp::RenameTable => model
            .candidate
            .table_refs()
            .into_iter()
            .filter(|(_, t)| *t == table.as_str())
            .map(|(f, _)| f)
            .collect(),
        _ => {
            let col = from.as_deref().unwrap_or("");
            model
                .candidate
                .column_refs()
                .into_iter()
                .filter(|(_, t, c)| *t == table && *c == col)
                .map(|(f, _, _)| f)
                .collect()
        }
    }
}

fn finding(
    id: &str,
    evidence: Vec<Evidence>,
    impact: &str,
    release: Impact,
    rollback: Impact,
    recommendation: Option<&str>,
) -> Finding {
    let m = meta(id);
    Finding {
        rule_id: id.to_string(),
        severity: m.default_severity,
        category: m.category,
        title: m.title.to_string(),
        evidence,
        impact: impact.to_string(),
        release_impact: release,
        rollback_impact: rollback,
        recommendation: recommendation.map(|s| s.to_string()),
    }
}

fn db_backward(model: &ReleaseModel) -> Vec<Finding> {
    if !model.policy.require_backward_compatible || !is_rolling(model) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for change in breaking_schema(model) {
        let refs = prev_refs_for(model, change);
        if refs.is_empty() {
            continue;
        }
        let mut evidence = vec![ev(change, schema_label(change))];
        evidence.push(ev(
            refs[0],
            match &refs[0].fact {
                Fact::ColumnReference { table, column } => {
                    format!("Previous release references {table}.{column}")
                }
                Fact::TableReference { table } => {
                    format!("Previous release references table {table}")
                }
                _ => "Previous application reference".into(),
            },
        ));
        evidence.push(strategy_evidence(model));
        out.push(finding(
            "DB-BACKWARD-COMPAT-001",
            evidence,
            "Previous application version is incompatible with the candidate schema during rolling coexistence.",
            Impact::Unsafe,
            Impact::Unsafe,
            Some("Use expand/contract: add the new column, dual-write, deploy app, then drop the old column in a later release."),
        ));
    }
    out
}

fn db_rollback(model: &ReleaseModel) -> Vec<Finding> {
    if !model.policy.rollback_required {
        return Vec::new();
    }
    let mut out = Vec::new();
    for change in breaking_schema(model) {
        let refs = prev_refs_for(model, change);
        if refs.is_empty() {
            continue;
        }
        let mut evidence = vec![ev(change, schema_label(change))];
        evidence.push(ev(
            refs[0],
            match &refs[0].fact {
                Fact::ColumnReference { table, column } => {
                    format!("Previous release references {table}.{column}")
                }
                Fact::TableReference { table } => {
                    format!("Previous release references table {table}")
                }
                _ => "Previous application reference".into(),
            },
        ));
        evidence.push(policy_ev(
            model,
            "rollback.required: true (previous app + candidate schema)",
        ));
        out.push(finding(
            "DB-ROLLBACK-COMPAT-001",
            evidence,
            "Rolling back the application keeps the candidate schema, which previous code cannot use.",
            Impact::Unsafe,
            Impact::Unsafe,
            Some("Do not contract the schema until the previous application version is retired."),
        ));
    }
    out
}

fn db_notnull(model: &ReleaseModel) -> Vec<Finding> {
    let mut out = Vec::new();
    for change in model.new_schema_changes() {
        let Fact::SchemaChange {
            operation: SchemaOp::SetNotNull,
            table,
            from,
            ..
        } = &change.fact
        else {
            continue;
        };
        let refs = prev_refs_for(model, change);
        if refs.is_empty() {
            continue;
        }
        let mut evidence = vec![ev(
            change,
            format!(
                "SET NOT NULL on {}.{}",
                table,
                from.as_deref().unwrap_or("?")
            ),
        )];
        evidence.push(ev(
            refs[0],
            "Previous application still uses this column/table",
        ));
        if is_rolling(model) {
            evidence.push(strategy_evidence(model));
        }
        out.push(finding(
            "DB-NOTNULL-COMPAT-001",
            evidence,
            "Previous application writes may violate the new NOT NULL constraint.",
            if is_rolling(model) {
                Impact::Unsafe
            } else {
                Impact::Review
            },
            Impact::Unsafe,
            Some("Backfill, then set NOT NULL in a later release after both versions write non-null values."),
        ));
    }
    out
}

fn db_add_notnull(model: &ReleaseModel) -> Vec<Finding> {
    let mut out = Vec::new();
    for change in model.new_schema_changes() {
        let Fact::SchemaChange {
            operation: SchemaOp::AddNotNullColumn,
            table,
            to,
            ..
        } = &change.fact
        else {
            continue;
        };
        let refs = prev_refs_for(model, change);
        if refs.is_empty() {
            continue;
        }
        out.push(finding(
            "DB-ADD-NOTNULL-001",
            vec![
                ev(
                    change,
                    format!(
                        "ADD COLUMN {} NOT NULL without DEFAULT on {table}",
                        to.as_deref().unwrap_or("?")
                    ),
                ),
                ev(
                    refs[0],
                    format!("Previous application references table {table}"),
                ),
            ],
            "Previous inserts that omit the new column will fail.",
            Impact::Unsafe,
            Impact::Unsafe,
            Some("Add the column as nullable or with DEFAULT first (expand)."),
        ));
    }
    out
}

fn db_type(model: &ReleaseModel) -> Vec<Finding> {
    let mut out = Vec::new();
    for change in model.new_schema_changes() {
        let Fact::SchemaChange {
            operation: SchemaOp::TypeChange,
            ..
        } = &change.fact
        else {
            continue;
        };
        let refs = prev_refs_for(model, change);
        if refs.is_empty() {
            continue;
        }
        out.push(finding(
            "DB-TYPE-COMPAT-001",
            vec![
                ev(change, schema_label(change)),
                ev(refs[0], "Previous application references this column"),
            ],
            "Type change can break previous application reads/writes and rollback.",
            Impact::Unsafe,
            Impact::Unsafe,
            Some("Add a new column with the new type (expand/contract) instead of in-place type change."),
        ));
    }
    out
}

fn db_candidate_app(model: &ReleaseModel) -> Vec<Finding> {
    let mut out = Vec::new();
    for change in breaking_schema(model) {
        let refs = cand_refs_for(model, change);
        if refs.is_empty() {
            continue;
        }
        out.push(finding(
            "DB-CANDIDATE-APP-001",
            vec![
                ev(change, schema_label(change)),
                ev(
                    refs[0],
                    "Candidate application still references the old object",
                ),
            ],
            "Head application is inconsistent with the candidate schema.",
            Impact::Unsafe,
            Impact::Unsafe,
            None,
        ));
    }
    out
}

fn db_expand_hint(model: &ReleaseModel) -> Vec<Finding> {
    // Distinct from DB-BACKWARD only when rename happens and previous still uses old column
    // (same evidence). Keep it as a dedicated rule id used by demo explain text; skip if
    // backward already covers the same change to avoid duplicate noise.
    // GOAL wants expand/contract counterpart to PASS — that is absence of rename.
    // This rule fires only on rename_column with previous refs (overlaps backward).
    // We keep it as additional title-level detail only when rolling.
    let mut out = Vec::new();
    if !is_rolling(model) {
        return out;
    }
    for change in model.new_schema_changes() {
        let Fact::SchemaChange {
            operation: SchemaOp::RenameColumn,
            ..
        } = &change.fact
        else {
            continue;
        };
        let refs = prev_refs_for(model, change);
        if refs.is_empty() {
            continue;
        }
        // Skip duplicate of backward if policy already requires it — still useful as
        // explicit expand/contract evidence. Deduplicate in engine by allowing both
        // (GOAL example uses a single finding). Prefer not double-failing: only emit
        // this if backward compat is overridden off.
        if model.policy.require_backward_compatible {
            continue;
        }
        out.push(finding(
            "DB-EXPAND-CONTRACT-HINT-001",
            vec![
                ev(change, schema_label(change)),
                ev(refs[0], "Previous app uses old column"),
            ],
            "Single-step rename is not expand/contract.",
            Impact::Unsafe,
            Impact::Unsafe,
            Some("ADD COLUMN email_address, backfill, switch app, then DROP COLUMN email."),
        ));
    }
    out
}

fn candidate_contract_keys(model: &ReleaseModel) -> Vec<String> {
    let mut keys = model.candidate.env_keys();
    keys.sort();
    keys.dedup();
    keys
}

fn config_rollback(model: &ReleaseModel) -> Vec<Finding> {
    if !model.policy.rollback_required {
        return Vec::new();
    }
    let contract = candidate_contract_keys(model);
    if contract.is_empty() && model.candidate.deploy_env_keys().is_empty() {
        // No config/deploy facts at all → do not guess.
        if model.candidate.dotenv_keys().is_empty() {
            return Vec::new();
        }
    }
    let mut out = Vec::new();
    for (loc, key) in model.previous.env_refs() {
        if contract.iter().any(|k| k == key) {
            continue;
        }
        // Need evidence that candidate actually has a config/deploy contract (not empty repo).
        let Some(contract_loc) = model
            .candidate
            .facts
            .iter()
            .find(|f| matches!(f.fact, Fact::EnvDefinition { .. }))
        else {
            continue;
        };
        out.push(finding(
            "CONFIG-ROLLBACK-COMPAT-001",
            vec![
                ev(loc, format!("Previous release requires {key}")),
                ev(
                    contract_loc,
                    format!(
                        "Candidate config/deploy contract provides: {}",
                        contract.join(", ")
                    ),
                ),
            ],
            "Rollback restores previous application against candidate env contract.",
            Impact::Unsafe,
            Impact::Unsafe,
            Some("Keep the old variable until the previous application version is gone (expand/contract for config)."),
        ));
    }
    out
}

fn config_forward(model: &ReleaseModel) -> Vec<Finding> {
    let contract = candidate_contract_keys(model);
    if contract.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (loc, key) in model.candidate.env_refs() {
        if contract.iter().any(|k| k == key) {
            continue;
        }
        let contract_loc = model
            .candidate
            .facts
            .iter()
            .find(|f| matches!(f.fact, Fact::EnvDefinition { .. }))
            .expect("contract non-empty");
        out.push(finding(
            "CONFIG-FORWARD-COMPAT-001",
            vec![
                ev(loc, format!("Candidate application requires {key}")),
                ev(
                    contract_loc,
                    format!("Candidate contract keys: {}", contract.join(", ")),
                ),
            ],
            "New application version will start without a required environment variable.",
            Impact::Unsafe,
            Impact::Review,
            Some("Add the variable to the deployment spec or .env.example in the same release."),
        ));
    }
    out
}

fn config_shared_rolling(model: &ReleaseModel) -> Vec<Finding> {
    if !is_rolling(model) {
        return Vec::new();
    }
    let prev_dot = model.previous.dotenv_keys();
    let cand_dot = model.candidate.dotenv_keys();
    if prev_dot.is_empty() || cand_dot.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (loc, key) in model.previous.env_refs() {
        if prev_dot.iter().any(|k| k == key) && !cand_dot.iter().any(|k| k == key) {
            let Some(dot) = model.candidate.facts.iter().find(|f| {
                matches!(
                    f.fact,
                    Fact::EnvDefinition {
                        source: crate::fact::EnvSource::DotEnv,
                        ..
                    }
                )
            }) else {
                continue;
            };
            let mut evidence = vec![
                ev(loc, format!("Previous application requires {key}")),
                ev(
                    dot,
                    format!("Candidate shared dotenv keys: {}", cand_dot.join(", ")),
                ),
                strategy_evidence(model),
            ];
            let _ = &mut evidence;
            out.push(finding(
                "CONFIG-SHARED-ROLLING-001",
                evidence,
                "Shared dotenv is one contract for mixed versions during rolling.",
                Impact::Unsafe,
                Impact::Unsafe,
                Some("Leave the old key in the shared env file until old pods are gone."),
            ));
        }
    }
    out
}

fn deploy_replica(model: &ReleaseModel) -> Vec<Finding> {
    let Some(min) = model.policy.minimum_replicas else {
        return Vec::new();
    };
    let Some(loc) = model.candidate.replicas() else {
        return Vec::new();
    };
    let Fact::ReplicaCount { count } = loc.fact else {
        return Vec::new();
    };
    if count >= min {
        return Vec::new();
    }
    vec![finding(
        "DEPLOY-REPLICA-001",
        vec![
            ev(loc, format!("Candidate replicas: {count}")),
            policy_ev(model, format!("Required minimum: {min}")),
        ],
        "Availability policy is not met.",
        Impact::Unsafe,
        Impact::Review,
        Some("Raise replicas to at least the policy minimum."),
    )]
}

fn deploy_replica_regression(model: &ReleaseModel) -> Vec<Finding> {
    if !model.policy.deny_replica_regression {
        return Vec::new();
    }
    let Some(prev) = model.previous.replica_count() else {
        return Vec::new();
    };
    let Some(loc) = model.candidate.replicas() else {
        return Vec::new();
    };
    let Fact::ReplicaCount { count } = loc.fact else {
        return Vec::new();
    };
    if count >= prev {
        return Vec::new();
    }
    let prev_loc = model.previous.replicas().unwrap();
    vec![finding(
        "DEPLOY-REPLICA-REGRESSION-001",
        vec![
            ev(prev_loc, format!("Previous replicas: {prev}")),
            ev(loc, format!("Candidate replicas: {count}")),
            policy_ev(model, "availability.deny_regression: true"),
        ],
        "Replica count decreased versus the previous manifest.",
        Impact::Unsafe,
        Impact::Review,
        None,
    )]
}

fn deploy_replicas_zero(model: &ReleaseModel) -> Vec<Finding> {
    let Some(loc) = model.candidate.replicas() else {
        return Vec::new();
    };
    let Fact::ReplicaCount { count: 0 } = loc.fact else {
        return Vec::new();
    };
    vec![finding(
        "DEPLOY-REPLICAS-ZERO-001",
        vec![ev(loc, "Candidate replicas: 0")],
        "Service will have no ready pods.",
        Impact::Unsafe,
        Impact::Unsafe,
        Some("Keep at least one replica, or scale via an explicit maintenance policy (not V0.1)."),
    )]
}

fn deploy_mem(model: &ReleaseModel) -> Vec<Finding> {
    let Some(min) = model.policy.minimum_memory.as_deref() else {
        return Vec::new();
    };
    let Some(min_b) = memory_bytes(min) else {
        return Vec::new();
    };
    let Some(loc) = model.candidate.resource() else {
        return Vec::new();
    };
    let Fact::ResourceLimit {
        memory: Some(ref mem),
        ..
    } = loc.fact
    else {
        return Vec::new();
    };
    let Some(got) = memory_bytes(mem) else {
        return Vec::new();
    };
    if got >= min_b {
        return Vec::new();
    }
    vec![finding(
        "DEPLOY-RESOURCE-MEM-001",
        vec![
            ev(loc, format!("Candidate memory limit: {mem}")),
            policy_ev(model, format!("Required minimum: {min}")),
        ],
        "Memory limit is below the explicit policy floor.",
        Impact::Unsafe,
        Impact::Review,
        None,
    )]
}

fn deploy_cpu(model: &ReleaseModel) -> Vec<Finding> {
    let Some(min) = model.policy.minimum_cpu.as_deref() else {
        return Vec::new();
    };
    let Some(min_m) = cpu_millis(min) else {
        return Vec::new();
    };
    let Some(loc) = model.candidate.resource() else {
        return Vec::new();
    };
    let Fact::ResourceLimit {
        cpu: Some(ref cpu), ..
    } = loc.fact
    else {
        return Vec::new();
    };
    let Some(got) = cpu_millis(cpu) else {
        return Vec::new();
    };
    if got >= min_m {
        return Vec::new();
    }
    vec![finding(
        "DEPLOY-RESOURCE-CPU-001",
        vec![
            ev(loc, format!("Candidate CPU limit: {cpu}")),
            policy_ev(model, format!("Required minimum: {min}")),
        ],
        "CPU limit is below the explicit policy floor.",
        Impact::Unsafe,
        Impact::Review,
        None,
    )]
}

fn deploy_resource_regression(model: &ReleaseModel) -> Vec<Finding> {
    if !model.policy.deny_resource_regression {
        return Vec::new();
    }
    let Some(prev) = model.previous.resource() else {
        return Vec::new();
    };
    let Some(cand) = model.candidate.resource() else {
        return Vec::new();
    };
    let (
        Fact::ResourceLimit {
            cpu: ref pcpu,
            memory: ref pmem,
        },
        Fact::ResourceLimit {
            cpu: ref ccpu,
            memory: ref cmem,
        },
    ) = (&prev.fact, &cand.fact)
    else {
        return Vec::new();
    };
    let mut dropped = false;
    if let (Some(p), Some(c)) = (
        pmem.as_deref().and_then(memory_bytes),
        cmem.as_deref().and_then(memory_bytes),
    ) {
        if c < p {
            dropped = true;
        }
    }
    if let (Some(p), Some(c)) = (
        pcpu.as_deref().and_then(cpu_millis),
        ccpu.as_deref().and_then(cpu_millis),
    ) {
        if c < p {
            dropped = true;
        }
    }
    if !dropped {
        return Vec::new();
    }
    vec![finding(
        "DEPLOY-RESOURCE-REGRESSION-001",
        vec![
            ev(
                prev,
                format!("Previous limits cpu={pcpu:?} memory={pmem:?}"),
            ),
            ev(
                cand,
                format!("Candidate limits cpu={ccpu:?} memory={cmem:?}"),
            ),
            policy_ev(model, "resources.deny_regression: true"),
        ],
        "Candidate compute limits are lower than the previous manifest.",
        Impact::Unsafe,
        Impact::Review,
        None,
    )]
}

fn deploy_recreate(model: &ReleaseModel) -> Vec<Finding> {
    if model.policy.minimum_replicas.is_none() {
        return Vec::new();
    }
    if model.candidate.strategy(&model.policy) != StrategyKind::Recreate {
        return Vec::new();
    }
    let loc = model
        .candidate
        .strategy_fact()
        .cloned()
        .unwrap_or_else(|| LocatedFact {
            path: model
                .policy_path
                .clone()
                .unwrap_or_else(|| ".coexistgate.yml".into()),
            line: 1,
            fact: Fact::DeploymentStrategy {
                kind: StrategyKind::Recreate,
            },
        });
    vec![finding(
        "DEPLOY-RECREATE-DOWNTIME-001",
        vec![
            ev(&loc, "Deployment strategy Recreate"),
            policy_ev(
                model,
                format!(
                    "availability.minimum_replicas: {}",
                    model.policy.minimum_replicas.unwrap()
                ),
            ),
        ],
        "Recreate terminates all pods before creating new ones, violating the availability floor during rollout.",
        Impact::Unsafe,
        Impact::Review,
        Some("Use RollingUpdate if the availability policy must hold during deploys."),
    )]
}

fn deploy_max_unavail(model: &ReleaseModel) -> Vec<Finding> {
    if model.policy.minimum_replicas.is_none() {
        return Vec::new();
    }
    let Some(loc) = model.candidate.max_unavailable() else {
        return Vec::new();
    };
    let Fact::MaxUnavailable { ref spec } = loc.fact else {
        return Vec::new();
    };
    let replicas = model.candidate.replica_count().unwrap_or(1);
    let drops_all = spec.trim() == "100%"
        || spec
            .trim()
            .parse::<u32>()
            .ok()
            .is_some_and(|n| n >= replicas);
    if !drops_all {
        return Vec::new();
    }
    vec![finding(
        "DEPLOY-MAX-UNAVAILABLE-001",
        vec![
            ev(loc, format!("maxUnavailable: {spec}")),
            policy_ev(
                model,
                format!(
                    "availability.minimum_replicas: {}",
                    model.policy.minimum_replicas.unwrap()
                ),
            ),
        ],
        "Rolling update may remove every replica.",
        Impact::Unsafe,
        Impact::Review,
        Some("Set maxUnavailable so remaining pods still meet the availability floor."),
    )]
}

pub fn memory_bytes(s: &str) -> Option<u64> {
    let s = s.trim();
    let digits: String = s
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let unit: String = s
        .chars()
        .skip_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let n: f64 = digits.parse().ok()?;
    let mul = match unit.trim().to_ascii_lowercase().as_str() {
        "" | "b" => 1.0,
        "k" | "kb" => 1000.0,
        "ki" | "kib" => 1024.0,
        "m" | "mb" => 1_000_000.0,
        "mi" | "mib" => 1024.0 * 1024.0,
        "g" | "gb" => 1_000_000_000.0,
        "gi" | "gib" => 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some((n * mul) as u64)
}

pub fn cpu_millis(s: &str) -> Option<u64> {
    let s = s.trim();
    if let Some(m) = s.strip_suffix('m') {
        return m.parse().ok();
    }
    let n: f64 = s.parse().ok()?;
    Some((n * 1000.0) as u64)
}

#[allow(dead_code)]
fn _version() -> &'static str {
    ENGINE_VERSION
}
