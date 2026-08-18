use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Critical => "critical",
            Severity::High => "high",
            Severity::Medium => "medium",
            Severity::Low => "low",
            Severity::Info => "info",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "critical" => Some(Self::Critical),
            "high" => Some(Self::High),
            "medium" => Some(Self::Medium),
            "low" => Some(Self::Low),
            "info" => Some(Self::Info),
            _ => None,
        }
    }

    /// Lowest listed `fail_on` is the threshold; that severity **or worse** blocks.
    pub fn blocks_gate(self, fail_on: &[Severity]) -> bool {
        fail_on.iter().copied().min().is_some_and(|th| self >= th)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Compatibility,
    Rollback,
    Availability,
    Configuration,
    Resource,
}

impl Category {
    pub fn as_str(self) -> &'static str {
        match self {
            Category::Compatibility => "compatibility",
            Category::Rollback => "rollback",
            Category::Availability => "availability",
            Category::Configuration => "configuration",
            Category::Resource => "resource",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Impact {
    Safe,
    Unsafe,
    Review,
}

impl Impact {
    pub fn as_str(self) -> &'static str {
        match self {
            Impact::Safe => "safe",
            Impact::Unsafe => "unsafe",
            Impact::Review => "review",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Evidence {
    pub artifact: String,
    pub line: u32,
    pub fact: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Finding {
    pub rule_id: String,
    pub severity: Severity,
    pub category: Category,
    pub title: String,
    pub evidence: Vec<Evidence>,
    pub impact: String,
    pub release_impact: Impact,
    pub rollback_impact: Impact,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<String>,
}

impl Finding {
    pub fn sort_key(&self) -> (&str, &str, u32) {
        let (path, line) = self
            .evidence
            .first()
            .map(|e| (e.artifact.as_str(), e.line))
            .unwrap_or(("", 0));
        (self.rule_id.as_str(), path, line)
    }
}
