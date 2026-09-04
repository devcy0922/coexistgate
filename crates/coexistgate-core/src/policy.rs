use serde::{Deserialize, Serialize};

use crate::finding::Severity;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Policy {
    pub version: u32,
    pub release_strategy: Option<StrategyHint>,
    pub rollback_required: bool,
    pub require_backward_compatible: bool,
    pub minimum_replicas: Option<u32>,
    pub deny_replica_regression: bool,
    pub minimum_memory: Option<String>,
    pub minimum_cpu: Option<String>,
    pub deny_resource_regression: bool,
    pub fail_on: Vec<Severity>,
    pub rule_overrides: Vec<RuleOverride>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrategyHint {
    Rolling,
    Recreate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleOverride {
    pub rule_id: String,
    pub level: OverrideLevel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverrideLevel {
    Error,
    Warning,
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FailOn;

impl Default for Policy {
    fn default() -> Self {
        Self {
            version: 1,
            release_strategy: Some(StrategyHint::Rolling),
            rollback_required: true,
            require_backward_compatible: true,
            minimum_replicas: None,
            deny_replica_regression: false,
            minimum_memory: None,
            minimum_cpu: None,
            deny_resource_regression: false,
            fail_on: vec![Severity::Critical, Severity::High],
            rule_overrides: Vec::new(),
        }
    }
}

impl Policy {
    pub fn from_yaml(text: &str) -> crate::Result<Self> {
        let raw: RawPolicy =
            serde_yaml::from_str(text).map_err(|e| crate::Error::Policy(e.to_string()))?;
        raw.try_into()
    }

    pub fn override_for(&self, rule_id: &str) -> Option<OverrideLevel> {
        self.rule_overrides
            .iter()
            .find(|o| o.rule_id == rule_id)
            .map(|o| o.level)
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct RawPolicy {
    #[serde(default = "one")]
    version: u32,
    #[serde(default)]
    release: Option<RawRelease>,
    #[serde(default)]
    rollback: Option<RawRollback>,
    #[serde(default)]
    compatibility: Option<RawCompat>,
    #[serde(default)]
    availability: Option<RawAvailability>,
    #[serde(default)]
    resources: Option<RawResources>,
    #[serde(default)]
    gate: Option<RawGate>,
    #[serde(default)]
    rules: Option<std::collections::BTreeMap<String, String>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RawRelease {
    strategy: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RawRollback {
    required: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RawCompat {
    require_backward_compatible: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RawAvailability {
    minimum_replicas: Option<u32>,
    deny_regression: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RawResources {
    minimum_memory: Option<String>,
    minimum_cpu: Option<String>,
    deny_regression: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RawGate {
    fail_on: Option<Vec<String>>,
}

fn one() -> u32 {
    1
}

impl TryFrom<RawPolicy> for Policy {
    type Error = crate::Error;

    fn try_from(raw: RawPolicy) -> crate::Result<Self> {
        let mut p = Policy {
            version: raw.version,
            ..Policy::default()
        };
        if let Some(r) = raw.release {
            p.release_strategy = match r.strategy.as_deref() {
                Some("rolling") | Some("RollingUpdate") => Some(StrategyHint::Rolling),
                Some("recreate") | Some("Recreate") => Some(StrategyHint::Recreate),
                Some(other) => {
                    return Err(crate::Error::Policy(format!(
                        "unknown release.strategy: {other}"
                    )))
                }
                None => p.release_strategy,
            };
        }
        if let Some(r) = raw.rollback {
            p.rollback_required = r.required.unwrap_or(true);
        }
        if let Some(c) = raw.compatibility {
            p.require_backward_compatible = c.require_backward_compatible.unwrap_or(true);
        }
        if let Some(a) = raw.availability {
            p.minimum_replicas = a.minimum_replicas;
            p.deny_replica_regression = a.deny_regression.unwrap_or(false);
        }
        if let Some(r) = raw.resources {
            p.minimum_memory = r.minimum_memory;
            p.minimum_cpu = r.minimum_cpu;
            p.deny_resource_regression = r.deny_regression.unwrap_or(false);
        }
        if let Some(g) = raw.gate {
            if let Some(list) = g.fail_on {
                p.fail_on = list
                    .iter()
                    .map(|s| parse_severity(s))
                    .collect::<crate::Result<Vec<_>>>()?;
            }
        }
        if let Some(rules) = raw.rules {
            for (id, level) in rules {
                p.rule_overrides.push(RuleOverride {
                    rule_id: id,
                    level: parse_override(&level)?,
                });
            }
        }
        Ok(p)
    }
}

fn parse_severity(s: &str) -> crate::Result<Severity> {
    match s.to_ascii_lowercase().as_str() {
        "critical" => Ok(Severity::Critical),
        "high" => Ok(Severity::High),
        "medium" => Ok(Severity::Medium),
        "low" => Ok(Severity::Low),
        other => Err(crate::Error::Policy(format!("unknown severity: {other}"))),
    }
}

fn parse_override(s: &str) -> crate::Result<OverrideLevel> {
    match s.to_ascii_lowercase().as_str() {
        "error" => Ok(OverrideLevel::Error),
        "warning" | "warn" => Ok(OverrideLevel::Warning),
        "off" | "disable" | "disabled" => Ok(OverrideLevel::Off),
        other => Err(crate::Error::Policy(format!(
            "unknown rule override: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_example_policy() {
        let yaml = r#"
version: 1
release:
  strategy: rolling
rollback:
  required: true
compatibility:
  require_backward_compatible: true
availability:
  minimum_replicas: 2
gate:
  fail_on:
    - critical
    - high
rules:
  DB-BACKWARD-COMPAT-001: error
  DEPLOY-REPLICA-001: warning
"#;
        let p = Policy::from_yaml(yaml).unwrap();
        assert_eq!(p.minimum_replicas, Some(2));
        assert_eq!(
            p.override_for("DEPLOY-REPLICA-001"),
            Some(OverrideLevel::Warning)
        );
    }
}
