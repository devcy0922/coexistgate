use crate::analyzers::{builtin_analyzers, Analyzer};
use crate::discovery::classify;
use crate::fact::LocatedFact;
use crate::finding::Severity;
use crate::policy::Policy;
use crate::release_model::{ReleaseModel, SnapshotFacts};
use crate::report::{category_statuses, GateDecision, Report};
use crate::rules;
use crate::tree::FileTree;
use crate::ENGINE_VERSION;

#[derive(Clone, Debug, Default)]
pub struct AnalysisRequest {
    pub previous: FileTree,
    pub candidate: FileTree,
    pub policy: Option<Policy>,
    pub fail_on: Option<Vec<Severity>>,
}

pub fn analyze(request: AnalysisRequest) -> crate::Result<Report> {
    analyze_with_analyzers(request, &builtin_analyzers())
}

pub fn analyze_with_analyzers(
    request: AnalysisRequest,
    analyzers: &[Box<dyn Analyzer>],
) -> crate::Result<Report> {
    let (policy, policy_path) = match request.policy {
        Some(p) => (p, find_policy_path(&request.candidate).or_else(|| find_policy_path(&request.previous))),
        None => load_policy(&request.candidate, &request.previous)?,
    };
    let mut policy = policy;
    if let Some(fail_on) = request.fail_on {
        policy.fail_on = fail_on;
    }

    let t0 = std::time::Instant::now();
    let mut analyzer_ms = Vec::new();
    let mut previous_facts = Vec::new();
    let mut candidate_facts = Vec::new();
    for a in analyzers {
        let start = std::time::Instant::now();
        previous_facts.extend(a.analyze(&request.previous));
        candidate_facts.extend(a.analyze(&request.candidate));
        analyzer_ms.push((a.id().to_string(), start.elapsed().as_millis() as u64));
        tracing::info!(
            analyzer = a.id(),
            duration_ms = analyzer_ms.last().map(|(_, d)| *d).unwrap_or(0),
            "analyzer complete"
        );
    }
    previous_facts.sort_by(fact_ord);
    candidate_facts.sort_by(fact_ord);

    let model = ReleaseModel {
        previous: SnapshotFacts {
            facts: previous_facts,
        },
        candidate: SnapshotFacts {
            facts: candidate_facts,
        },
        policy: policy.clone(),
        policy_path,
    };

    let findings = rules::apply_policy(rules::evaluate(&model), &policy);
    let gate = gate_decision(&findings, &policy.fail_on);
    let _elapsed = t0.elapsed();

    Ok(Report {
        engine_version: ENGINE_VERSION.to_string(),
        gate,
        fail_on: policy.fail_on.clone(),
        findings: findings.clone(),
        categories: category_statuses(&findings, &policy.fail_on),
        analyzer_durations_ms: analyzer_ms,
    })
}

fn fact_ord(a: &LocatedFact, b: &LocatedFact) -> std::cmp::Ordering {
    (&a.path, a.line, format!("{:?}", a.fact)).cmp(&(&b.path, b.line, format!("{:?}", b.fact)))
}

fn load_policy(candidate: &FileTree, previous: &FileTree) -> crate::Result<(Policy, Option<String>)> {
    if let Some(path) = find_policy_path(candidate) {
        let text = &candidate.get(&path).unwrap().content;
        return Ok((Policy::from_yaml(text)?, Some(path)));
    }
    if let Some(path) = find_policy_path(previous) {
        let text = &previous.get(&path).unwrap().content;
        return Ok((Policy::from_yaml(text)?, Some(path)));
    }
    Ok((Policy::default(), None))
}

fn find_policy_path(tree: &FileTree) -> Option<String> {
    tree.paths()
        .find(|p| classify(p) == crate::discovery::ArtifactKind::Policy)
        .map(|s| s.to_string())
}

pub fn gate_decision(findings: &[crate::Finding], fail_on: &[Severity]) -> GateDecision {
    if findings.iter().any(|f| f.severity.blocks_gate(fail_on)) {
        GateDecision::Fail
    } else {
        GateDecision::Pass
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finding::Impact;

    fn tree(files: &[(&str, &str)]) -> FileTree {
        let mut t = FileTree::new();
        for (p, c) in files {
            t.insert(*p, *c).unwrap();
        }
        t
    }

    const POLICY: &str = r#"
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
  fail_on: [critical, high]
"#;

    #[test]
    fn unsafe_rename_is_blocked_with_evidence() {
        let previous = tree(&[
            (
                "src/user_repository.ts",
                "export const q = `SELECT users.email FROM users`;\n",
            ),
            (
                "deploy/deployment.yaml",
                "kind: Deployment\nspec:\n  replicas: 3\n  strategy:\n    type: RollingUpdate\n",
            ),
            (".coexistgate.yml", POLICY),
        ]);
        let candidate = tree(&[
            (
                "src/user_repository.ts",
                "export const q = `SELECT users.email_address FROM users`;\n",
            ),
            (
                "migrations/002_email.sql",
                "ALTER TABLE users RENAME COLUMN email TO email_address;\n",
            ),
            (
                "deploy/deployment.yaml",
                "kind: Deployment\nspec:\n  replicas: 3\n  strategy:\n    type: RollingUpdate\n",
            ),
            (".coexistgate.yml", POLICY),
        ]);
        let report = analyze(AnalysisRequest {
            previous,
            candidate,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(report.gate, GateDecision::Fail);
        assert!(report
            .findings
            .iter()
            .any(|f| f.rule_id == "DB-BACKWARD-COMPAT-001"));
        assert!(report
            .findings
            .iter()
            .any(|f| f.rule_id == "DB-ROLLBACK-COMPAT-001"));
        for f in &report.findings {
            if f.severity >= Severity::High {
                assert!(!f.evidence.is_empty());
                assert_ne!(f.release_impact, Impact::Safe);
            }
        }
        let high_only = analyze(AnalysisRequest {
            previous: tree(&[
                (
                    "src/user_repository.ts",
                    "export const q = `SELECT users.email FROM users`;\n",
                ),
                (
                    "deploy/deployment.yaml",
                    "kind: Deployment\nspec:\n  replicas: 3\n  strategy:\n    type: RollingUpdate\n",
                ),
                (".coexistgate.yml", POLICY),
            ]),
            candidate: tree(&[
                (
                    "src/user_repository.ts",
                    "export const q = `SELECT users.email_address FROM users`;\n",
                ),
                (
                    "migrations/002_email.sql",
                    "ALTER TABLE users RENAME COLUMN email TO email_address;\n",
                ),
                (
                    "deploy/deployment.yaml",
                    "kind: Deployment\nspec:\n  replicas: 3\n  strategy:\n    type: RollingUpdate\n",
                ),
                (".coexistgate.yml", POLICY),
            ]),
            fail_on: Some(vec![Severity::High]),
            policy: None,
        })
        .unwrap();
        assert_eq!(high_only.gate, GateDecision::Fail);
    }

    #[test]
    fn expand_only_add_column_passes() {
        let previous = tree(&[
            (
                "src/user_repository.ts",
                "export const q = `SELECT users.email FROM users`;\n",
            ),
            (
                "deploy/deployment.yaml",
                "kind: Deployment\nspec:\n  replicas: 3\n  strategy:\n    type: RollingUpdate\n",
            ),
            (".coexistgate.yml", POLICY),
        ]);
        let candidate = tree(&[
            (
                "src/user_repository.ts",
                "export const q = `SELECT users.email, users.email_address FROM users`;\n",
            ),
            (
                "migrations/002_email.sql",
                "ALTER TABLE users ADD COLUMN email_address TEXT;\n",
            ),
            (
                "deploy/deployment.yaml",
                "kind: Deployment\nspec:\n  replicas: 3\n  strategy:\n    type: RollingUpdate\n",
            ),
            (".coexistgate.yml", POLICY),
        ]);
        let report = analyze(AnalysisRequest {
            previous,
            candidate,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(report.gate, GateDecision::Pass);
        assert!(!report
            .findings
            .iter()
            .any(|f| f.rule_id.starts_with("DB-BACKWARD")));
    }

    #[test]
    fn availability_regression() {
        let previous = tree(&[
            (
                "deploy/deployment.yaml",
                "kind: Deployment\nspec:\n  replicas: 3\n",
            ),
            (".coexistgate.yml", POLICY),
        ]);
        let candidate = tree(&[
            (
                "deploy/deployment.yaml",
                "kind: Deployment\nspec:\n  replicas: 1\n",
            ),
            (".coexistgate.yml", POLICY),
        ]);
        let report = analyze(AnalysisRequest {
            previous,
            candidate,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(report.gate, GateDecision::Fail);
        assert!(report
            .findings
            .iter()
            .any(|f| f.rule_id == "DEPLOY-REPLICA-001"));
    }

    #[test]
    fn config_rollback_break() {
        let previous = tree(&[
            ("src/cache.ts", "const u = process.env.REDIS_URL;\n"),
            (".env.example", "REDIS_URL=\n"),
            (
                "deploy/deployment.yaml",
                "kind: Deployment\nspec:\n  replicas: 2\n  template:\n    spec:\n      containers:\n        - name: app\n          env:\n            - name: REDIS_URL\n              value: redis://x\n",
            ),
            (".coexistgate.yml", POLICY),
        ]);
        let candidate = tree(&[
            ("src/cache.ts", "const u = process.env.CACHE_URL;\n"),
            (".env.example", "CACHE_URL=\n"),
            (
                "deploy/deployment.yaml",
                "kind: Deployment\nspec:\n  replicas: 2\n  template:\n    spec:\n      containers:\n        - name: app\n          env:\n            - name: CACHE_URL\n              value: redis://x\n",
            ),
            (".coexistgate.yml", POLICY),
        ]);
        let report = analyze(AnalysisRequest {
            previous,
            candidate,
            ..Default::default()
        })
        .unwrap();
        assert!(report
            .findings
            .iter()
            .any(|f| f.rule_id == "CONFIG-ROLLBACK-COMPAT-001"));
        assert_eq!(report.gate, GateDecision::Fail);
    }

    #[test]
    fn deterministic() {
        let previous = tree(&[
            ("src/a.ts", "const x = users.email;\n"),
            ("deploy/deployment.yaml", "kind: Deployment\nspec:\n  replicas: 3\n  strategy:\n    type: RollingUpdate\n"),
            (".coexistgate.yml", POLICY),
        ]);
        let candidate = tree(&[
            ("src/a.ts", "const x = users.email_address;\n"),
            ("migrations/1.sql", "ALTER TABLE users DROP COLUMN email;\n"),
            ("deploy/deployment.yaml", "kind: Deployment\nspec:\n  replicas: 3\n  strategy:\n    type: RollingUpdate\n"),
            (".coexistgate.yml", POLICY),
        ]);
        let req = AnalysisRequest {
            previous: previous.clone(),
            candidate: candidate.clone(),
            ..Default::default()
        };
        let a = serde_json::to_string(&analyze(req.clone()).unwrap()).unwrap();
        let b = serde_json::to_string(&analyze(req).unwrap()).unwrap();
        assert_eq!(a, b);
    }
}
