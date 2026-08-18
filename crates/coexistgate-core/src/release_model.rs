use crate::fact::{EnvSource, Fact, LocatedFact, SchemaOp, StrategyKind};
use crate::policy::{Policy, StrategyHint};

#[derive(Clone, Debug)]
pub struct SnapshotFacts {
    pub facts: Vec<LocatedFact>,
}

impl SnapshotFacts {
    pub fn schema(&self) -> Vec<&LocatedFact> {
        self.facts
            .iter()
            .filter(|f| matches!(f.fact, Fact::SchemaChange { .. }))
            .collect()
    }

    pub fn column_refs(&self) -> Vec<(&LocatedFact, &str, &str)> {
        self.facts
            .iter()
            .filter_map(|f| match &f.fact {
                Fact::ColumnReference { table, column } => {
                    Some((f, table.as_str(), column.as_str()))
                }
                _ => None,
            })
            .collect()
    }

    pub fn table_refs(&self) -> Vec<(&LocatedFact, &str)> {
        self.facts
            .iter()
            .filter_map(|f| match &f.fact {
                Fact::TableReference { table } => Some((f, table.as_str())),
                _ => None,
            })
            .collect()
    }

    pub fn env_refs(&self) -> Vec<(&LocatedFact, &str)> {
        self.facts
            .iter()
            .filter_map(|f| match &f.fact {
                Fact::EnvReference { key } => Some((f, key.as_str())),
                _ => None,
            })
            .collect()
    }

    pub fn env_defs(&self, source: Option<EnvSource>) -> Vec<(&LocatedFact, &str)> {
        self.facts
            .iter()
            .filter_map(|f| match &f.fact {
                Fact::EnvDefinition { key, source: src } => {
                    if source.is_none() || source == Some(*src) {
                        Some((f, key.as_str()))
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect()
    }

    pub fn env_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self
            .facts
            .iter()
            .filter_map(|f| match &f.fact {
                Fact::EnvDefinition { key, .. } => Some(key.clone()),
                _ => None,
            })
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }

    pub fn dotenv_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self
            .facts
            .iter()
            .filter_map(|f| match &f.fact {
                Fact::EnvDefinition {
                    key,
                    source: EnvSource::DotEnv,
                } => Some(key.clone()),
                _ => None,
            })
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }

    pub fn deploy_env_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self
            .facts
            .iter()
            .filter_map(|f| match &f.fact {
                Fact::EnvDefinition {
                    key,
                    source: EnvSource::Kubernetes | EnvSource::Compose,
                } => Some(key.clone()),
                _ => None,
            })
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }

    pub fn strategy(&self, policy: &Policy) -> StrategyKind {
        self.facts
            .iter()
            .find_map(|f| match f.fact {
                Fact::DeploymentStrategy { kind } => Some(kind),
                _ => None,
            })
            .unwrap_or(match policy.release_strategy {
                Some(StrategyHint::Recreate) => StrategyKind::Recreate,
                _ => StrategyKind::Rolling,
            })
    }

    pub fn strategy_fact(&self) -> Option<&LocatedFact> {
        self.facts
            .iter()
            .find(|f| matches!(f.fact, Fact::DeploymentStrategy { .. }))
    }

    pub fn replicas(&self) -> Option<&LocatedFact> {
        self.facts
            .iter()
            .find(|f| matches!(f.fact, Fact::ReplicaCount { .. }))
    }

    pub fn replica_count(&self) -> Option<u32> {
        self.facts.iter().find_map(|f| match f.fact {
            Fact::ReplicaCount { count } => Some(count),
            _ => None,
        })
    }

    pub fn resource(&self) -> Option<&LocatedFact> {
        self.facts
            .iter()
            .find(|f| matches!(f.fact, Fact::ResourceLimit { .. }))
    }

    pub fn max_unavailable(&self) -> Option<&LocatedFact> {
        self.facts
            .iter()
            .find(|f| matches!(f.fact, Fact::MaxUnavailable { .. }))
    }
}

#[derive(Clone, Debug)]
pub struct ReleaseModel {
    pub previous: SnapshotFacts,
    pub candidate: SnapshotFacts,
    pub policy: Policy,
    pub policy_path: Option<String>,
}

impl ReleaseModel {
    pub fn new_schema_changes(&self) -> Vec<&LocatedFact> {
        self.candidate
            .schema()
            .into_iter()
            .filter(|c| {
                !self.previous.schema().iter().any(|p| schema_eq(&p.fact, &c.fact) && p.path == c.path)
                    && !self
                        .previous
                        .schema()
                        .iter()
                        .any(|p| schema_eq(&p.fact, &c.fact))
            })
            .collect()
    }
}

fn schema_eq(a: &Fact, b: &Fact) -> bool {
    match (a, b) {
        (
            Fact::SchemaChange {
                table: t1,
                operation: o1,
                from: f1,
                to: to1,
                raw: r1,
                ..
            },
            Fact::SchemaChange {
                table: t2,
                operation: o2,
                from: f2,
                to: to2,
                raw: r2,
                ..
            },
        ) => t1 == t2 && o1 == o2 && f1 == f2 && to1 == to2 && collapse(r1) == collapse(r2),
        _ => false,
    }
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_ascii_lowercase()
}

pub fn schema_targets_ref(
    change: &Fact,
    table: &str,
    column: Option<&str>,
) -> bool {
    match change {
        Fact::SchemaChange {
            table: t,
            operation,
            from,
            to: _,
            ..
        } => {
            if t != table && from.as_deref() != Some(table) {
                return false;
            }
            match operation {
                SchemaOp::DropTable | SchemaOp::RenameTable => true,
                SchemaOp::DropColumn
                | SchemaOp::RenameColumn
                | SchemaOp::SetNotNull
                | SchemaOp::TypeChange => column.is_some_and(|c| from.as_deref() == Some(c)),
                SchemaOp::AddNotNullColumn => false,
            }
        }
        _ => false,
    }
}
