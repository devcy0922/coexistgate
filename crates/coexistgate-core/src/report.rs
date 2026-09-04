use serde::{Deserialize, Serialize};

use crate::finding::{Category, Finding, Severity};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GateDecision {
    Pass,
    Fail,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CategoryStatus {
    #[default]
    Pass,
    Fail,
    Warn,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Report {
    pub engine_version: String,
    pub gate: GateDecision,
    pub fail_on: Vec<Severity>,
    pub findings: Vec<Finding>,
    pub categories: CategoryBreakdown,
    #[serde(skip)]
    pub analyzer_durations_ms: Vec<(String, u64)>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CategoryBreakdown {
    pub compatibility: CategoryStatus,
    pub rollback: CategoryStatus,
    pub availability: CategoryStatus,
    pub configuration: CategoryStatus,
    pub resource: CategoryStatus,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

impl Report {
    pub fn counts(&self) -> (usize, usize, usize, usize) {
        (
            count(&self.findings, Severity::Critical),
            count(&self.findings, Severity::High),
            count(&self.findings, Severity::Medium),
            count(&self.findings, Severity::Low),
        )
    }
}

fn count(findings: &[Finding], sev: Severity) -> usize {
    findings.iter().filter(|f| f.severity == sev).count()
}

pub fn category_statuses(findings: &[Finding], fail_on: &[Severity]) -> CategoryBreakdown {
    fn status(findings: &[Finding], cat: Category, fail_on: &[Severity]) -> CategoryStatus {
        let in_cat: Vec<_> = findings.iter().filter(|f| f.category == cat).collect();
        if in_cat.iter().any(|f| f.severity.blocks_gate(fail_on)) {
            CategoryStatus::Fail
        } else if in_cat.iter().any(|f| f.severity >= Severity::Medium) {
            CategoryStatus::Warn
        } else {
            CategoryStatus::Pass
        }
    }
    CategoryBreakdown {
        compatibility: status(findings, Category::Compatibility, fail_on),
        rollback: status(findings, Category::Rollback, fail_on),
        availability: status(findings, Category::Availability, fail_on),
        configuration: status(findings, Category::Configuration, fail_on),
        resource: status(findings, Category::Resource, fail_on),
        critical: count(findings, Severity::Critical),
        high: count(findings, Severity::High),
        medium: count(findings, Severity::Medium),
        low: count(findings, Severity::Low),
    }
}

pub fn render_human(report: &Report) -> String {
    let mut s = String::new();
    let gate = match report.gate {
        GateDecision::Pass => "PASS",
        GateDecision::Fail => "FAIL",
    };
    s.push_str(&format!("CoexistGate Release Safety    {gate}\n\n"));
    s.push_str(&format!(
        "Compatibility      {}\nRollback           {}\nAvailability       {}\nConfiguration      {}\nResource           {}\n\n",
        flag(report.categories.compatibility),
        flag(report.categories.rollback),
        flag(report.categories.availability),
        flag(report.categories.configuration),
        flag(report.categories.resource),
    ));
    s.push_str(&format!(
        "Critical  {}\nHigh      {}\nMedium    {}\nLow       {}\n",
        report.categories.critical,
        report.categories.high,
        report.categories.medium,
        report.categories.low,
    ));
    if !report.findings.is_empty() {
        s.push('\n');
        for f in &report.findings {
            s.push_str(&format!(
                "[{}] {}  {}\n",
                f.severity.as_str().to_ascii_uppercase(),
                f.rule_id,
                f.title
            ));
            s.push_str(&format!("  impact: {}\n", f.impact));
            s.push_str(&format!(
                "  release: {}   rollback: {}\n",
                f.release_impact.as_str(),
                f.rollback_impact.as_str()
            ));
            for e in &f.evidence {
                s.push_str(&format!("  - {}:{}  {}\n", e.artifact, e.line, e.fact));
            }
            if let Some(r) = &f.recommendation {
                s.push_str(&format!("  recommendation: {r}\n"));
            }
            s.push('\n');
        }
    }
    s
}

fn flag(st: CategoryStatus) -> &'static str {
    match st {
        CategoryStatus::Pass => "PASS",
        CategoryStatus::Fail => "FAIL",
        CategoryStatus::Warn => "WARN",
    }
}

pub fn render_check_summary(report: &Report) -> String {
    format!(
        "Release Safety\n{}\n\nCompatibility      {}\nRollback           {}\nAvailability       {}\nConfiguration      {}\n\nCritical  {}\nHigh      {}\nMedium    {}\n",
        match report.gate {
            GateDecision::Pass => "PASSED",
            GateDecision::Fail => "FAILED",
        },
        flag(report.categories.compatibility),
        flag(report.categories.rollback),
        flag(report.categories.availability),
        flag(report.categories.configuration),
        report.categories.critical,
        report.categories.high,
        report.categories.medium,
    )
}
