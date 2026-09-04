use crate::analyzers::{AnalysisIssue, AnalysisOutput, Analyzer};
use crate::discovery::{files_of, looks_like_helm_template, ArtifactKind};
use crate::fact::{EnvSource, Fact, LocatedFact, StrategyKind};
use crate::tree::FileTree;

pub struct DeploymentAnalyzer;

impl Analyzer for DeploymentAnalyzer {
    fn id(&self) -> &'static str {
        "deployment"
    }

    fn analyze(&self, tree: &FileTree) -> AnalysisOutput {
        let mut out = Vec::new();
        let mut issues = Vec::new();
        for file in files_of(tree, ArtifactKind::Deployment) {
            if looks_like_helm_template(&file.content) {
                continue;
            }
            let name = file
                .path
                .rsplit('/')
                .next()
                .unwrap_or(&file.path)
                .to_ascii_lowercase();
            if name.contains("docker-compose") {
                if let Err(error) = serde_yaml::from_str::<serde_yaml::Value>(&file.content) {
                    issues.push(yaml_issue(&file.path, error.to_string()));
                } else if !file.content.contains("services:") {
                    issues.push(AnalysisIssue {
                        path: file.path.clone(),
                        line: 1,
                        message: "Docker Compose file has no services map".to_string(),
                    });
                }
                out.extend(parse_compose(&file.path, &file.content));
            } else {
                for doc in split_yaml_docs(&file.content) {
                    if let Err(error) = serde_yaml::from_str::<serde_yaml::Value>(doc.text) {
                        issues.push(yaml_issue(&file.path, error.to_string()));
                    }
                }
                out.extend(parse_k8s(&file.path, &file.content));
            }
        }
        out.sort_by(|a, b| (&a.path, a.line).cmp(&(&b.path, b.line)));
        AnalysisOutput { facts: out, issues }
    }
}

fn yaml_issue(path: &str, error: String) -> AnalysisIssue {
    AnalysisIssue {
        path: path.to_string(),
        line: 1,
        message: format!("malformed YAML: {error}"),
    }
}

fn parse_k8s(path: &str, content: &str) -> Vec<LocatedFact> {
    let mut facts = Vec::new();
    for doc in split_yaml_docs(content) {
        let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(doc.text) else {
            continue;
        };
        let Some(map) = value.as_mapping() else {
            continue;
        };
        let kind = map
            .get(serde_yaml::Value::String("kind".into()))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if kind != "Deployment" {
            continue;
        }
        let spec = map.get(serde_yaml::Value::String("spec".into()));
        if let Some(replicas) = spec.and_then(|s| s.get("replicas")).and_then(as_u32) {
            facts.push(LocatedFact {
                path: path.to_string(),
                line: line_of(doc.text, "replicas:"),
                fact: Fact::ReplicaCount { count: replicas },
            });
        }
        let strategy_type = spec
            .and_then(|s| s.get("strategy"))
            .and_then(|s| s.get("type"))
            .and_then(|v| v.as_str())
            .unwrap_or("RollingUpdate");
        let kind = if strategy_type.eq_ignore_ascii_case("Recreate") {
            StrategyKind::Recreate
        } else {
            StrategyKind::Rolling
        };
        facts.push(LocatedFact {
            path: path.to_string(),
            line: line_of(doc.text, "strategy:").max(1),
            fact: Fact::DeploymentStrategy { kind },
        });
        if let Some(max_unavail) = spec
            .and_then(|s| s.get("strategy"))
            .and_then(|s| s.get("rollingUpdate"))
            .and_then(|s| s.get("maxUnavailable"))
        {
            let spec_s = display_yaml(max_unavail);
            facts.push(LocatedFact {
                path: path.to_string(),
                line: line_of(doc.text, "maxUnavailable:"),
                fact: Fact::MaxUnavailable { spec: spec_s },
            });
        }
        if let Some(containers) = spec
            .and_then(|s| s.get("template"))
            .and_then(|s| s.get("spec"))
            .and_then(|s| s.get("containers"))
            .and_then(|v| v.as_sequence())
        {
            for c in containers {
                if let Some(limits) = c.get("resources").and_then(|r| r.get("limits")) {
                    let cpu = limits.get("cpu").and_then(as_string);
                    let memory = limits.get("memory").and_then(as_string);
                    if cpu.is_some() || memory.is_some() {
                        facts.push(LocatedFact {
                            path: path.to_string(),
                            line: line_of(doc.text, "limits:"),
                            fact: Fact::ResourceLimit { cpu, memory },
                        });
                    }
                }
                if let Some(env) = c.get("env").and_then(|v| v.as_sequence()) {
                    for item in env {
                        if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                            facts.push(LocatedFact {
                                path: path.to_string(),
                                line: line_of(doc.text, &format!("name: {name}")),
                                fact: Fact::EnvDefinition {
                                    key: name.to_string(),
                                    source: EnvSource::Kubernetes,
                                },
                            });
                        }
                    }
                }
            }
        }
    }
    facts
}

fn parse_compose(path: &str, content: &str) -> Vec<LocatedFact> {
    let mut facts = Vec::new();
    let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(content) else {
        return facts;
    };
    let Some(services) = value.get("services").and_then(|v| v.as_mapping()) else {
        return facts;
    };
    let mut saw_replicas = false;
    for (_name, svc) in services {
        if let Some(n) = svc
            .get("deploy")
            .and_then(|d| d.get("replicas"))
            .and_then(as_u32)
        {
            saw_replicas = true;
            facts.push(LocatedFact {
                path: path.to_string(),
                line: line_of(content, "replicas:"),
                fact: Fact::ReplicaCount { count: n },
            });
        }
        if let Some(order) = svc
            .get("deploy")
            .and_then(|d| d.get("update_config"))
            .and_then(|u| u.get("order"))
            .and_then(|v| v.as_str())
        {
            let kind = if order.contains("stop-first") {
                StrategyKind::Recreate
            } else {
                StrategyKind::Rolling
            };
            facts.push(LocatedFact {
                path: path.to_string(),
                line: line_of(content, "order:"),
                fact: Fact::DeploymentStrategy { kind },
            });
        }
        if let Some(limits) = svc
            .get("deploy")
            .and_then(|d| d.get("resources"))
            .and_then(|r| r.get("limits"))
        {
            let cpu = limits.get("cpus").and_then(as_string);
            let memory = limits.get("memory").and_then(as_string);
            if cpu.is_some() || memory.is_some() {
                facts.push(LocatedFact {
                    path: path.to_string(),
                    line: line_of(content, "limits:"),
                    fact: Fact::ResourceLimit { cpu, memory },
                });
            }
        }
        match svc.get("environment") {
            Some(serde_yaml::Value::Mapping(map)) => {
                for (k, _) in map {
                    if let Some(key) = k.as_str() {
                        facts.push(LocatedFact {
                            path: path.to_string(),
                            line: line_of(content, &format!("{key}:")),
                            fact: Fact::EnvDefinition {
                                key: key.to_string(),
                                source: EnvSource::Compose,
                            },
                        });
                    }
                }
            }
            Some(serde_yaml::Value::Sequence(seq)) => {
                for item in seq {
                    if let Some(s) = item.as_str() {
                        let key = s.split('=').next().unwrap_or(s).to_string();
                        facts.push(LocatedFact {
                            path: path.to_string(),
                            line: line_of(content, s),
                            fact: Fact::EnvDefinition {
                                key,
                                source: EnvSource::Compose,
                            },
                        });
                    }
                }
            }
            _ => {}
        }
    }
    if !saw_replicas {
        facts.push(LocatedFact {
            path: path.to_string(),
            line: 1,
            fact: Fact::ReplicaCount { count: 1 },
        });
    }
    if !facts
        .iter()
        .any(|f| matches!(f.fact, Fact::DeploymentStrategy { .. }))
    {
        facts.push(LocatedFact {
            path: path.to_string(),
            line: 1,
            fact: Fact::DeploymentStrategy {
                kind: StrategyKind::Rolling,
            },
        });
    }
    facts
}

struct YamlDoc<'a> {
    text: &'a str,
}

fn split_yaml_docs(content: &str) -> Vec<YamlDoc<'_>> {
    let mut docs = Vec::new();
    for chunk in content.split("\n---") {
        let text = chunk.trim_start_matches("---").trim();
        if !text.is_empty() {
            docs.push(YamlDoc { text });
        }
    }
    if docs.is_empty() && !content.trim().is_empty() {
        docs.push(YamlDoc {
            text: content.trim(),
        });
    }
    docs
}

fn line_of(text: &str, needle: &str) -> u32 {
    for (i, line) in text.lines().enumerate() {
        if line.contains(needle) {
            return (i as u32) + 1;
        }
    }
    1
}

fn as_u32(v: &serde_yaml::Value) -> Option<u32> {
    match v {
        serde_yaml::Value::Number(n) => n.as_u64().map(|x| x as u32).or_else(|| {
            n.as_i64()
                .and_then(|x| u32::try_from(x).ok())
                .or_else(|| n.as_f64().map(|f| f as u32))
        }),
        serde_yaml::Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn as_string(v: &serde_yaml::Value) -> Option<String> {
    match v {
        serde_yaml::Value::String(s) => Some(s.clone()),
        serde_yaml::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn display_yaml(v: &serde_yaml::Value) -> String {
    as_string(v).unwrap_or_else(|| format!("{v:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_k8s_rolling_and_replicas() {
        let mut t = FileTree::new();
        t.insert(
            "deploy/deployment.yaml",
            r#"
apiVersion: apps/v1
kind: Deployment
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
  template:
    spec:
      containers:
        - name: app
          env:
            - name: REDIS_URL
              value: redis://r
          resources:
            limits:
              memory: 2Gi
              cpu: "1"
"#,
        )
        .unwrap();
        let facts = DeploymentAnalyzer.analyze(&t).facts;
        assert!(facts
            .iter()
            .any(|f| matches!(f.fact, Fact::ReplicaCount { count: 3 })));
        assert!(facts.iter().any(|f| matches!(
            f.fact,
            Fact::DeploymentStrategy {
                kind: StrategyKind::Rolling
            }
        )));
        assert!(facts.iter().any(|f| matches!(
            &f.fact,
            Fact::EnvDefinition { key, .. } if key == "REDIS_URL"
        )));
    }
}
