use coexistgate_core::{render_check_summary, render_human, GateDecision, Report, CHECK_NAME};
use serde_json::{json, Value};

#[allow(dead_code)]
pub enum CheckConclusion {
    Success,
    Failure,
}

pub fn check_run_body(head_sha: &str, report: &Report) -> Value {
    let conclusion = match report.gate {
        GateDecision::Pass => "success",
        GateDecision::Fail => "failure",
    };
    let title = match report.gate {
        GateDecision::Pass => "Release Safety PASSED",
        GateDecision::Fail => "Release Safety FAILED",
    };
    let mut annotations = Vec::new();
    for f in report.findings.iter().take(50) {
        let Some(ev) = f.evidence.first() else {
            continue;
        };
        let level = match f.severity {
            coexistgate_core::Severity::Critical | coexistgate_core::Severity::High => "failure",
            coexistgate_core::Severity::Medium => "warning",
            _ => "notice",
        };
        annotations.push(json!({
            "path": ev.artifact,
            "start_line": ev.line.max(1),
            "end_line": ev.line.max(1),
            "annotation_level": level,
            "title": f.rule_id,
            "message": format!("{}\n{}", f.title, f.impact),
            "raw_details": ev.fact,
        }));
    }
    json!({
        "name": CHECK_NAME,
        "head_sha": head_sha,
        "status": "completed",
        "conclusion": conclusion,
        "output": {
            "title": title,
            "summary": render_check_summary(report),
            "text": render_human(report),
            "annotations": annotations,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use coexistgate_core::{
        Category, Evidence, Finding, GateDecision, Impact, Severity,
    };

    #[test]
    fn failure_conclusion() {
        let report = Report {
            engine_version: "0.1.0".into(),
            gate: GateDecision::Fail,
            fail_on: vec![Severity::High],
            findings: vec![Finding {
                rule_id: "DEPLOY-REPLICA-001".into(),
                severity: Severity::High,
                category: Category::Availability,
                title: "replicas".into(),
                evidence: vec![Evidence {
                    artifact: "deploy/deployment.yaml".into(),
                    line: 4,
                    fact: "replicas: 1".into(),
                }],
                impact: "too few".into(),
                release_impact: Impact::Unsafe,
                rollback_impact: Impact::Review,
                recommendation: None,
            }],
            categories: Default::default(),
            analyzer_durations_ms: vec![],
        };
        let body = check_run_body("abc", &report);
        assert_eq!(body["conclusion"], "failure");
        assert_eq!(body["name"], CHECK_NAME);
        assert_eq!(body["output"]["annotations"][0]["start_line"], 4);
    }
}
