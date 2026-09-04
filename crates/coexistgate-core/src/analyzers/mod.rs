mod application;
mod configuration;
mod deployment;
mod migration;

use crate::fact::LocatedFact;
use crate::tree::FileTree;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalysisIssue {
    pub path: String,
    pub line: u32,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AnalysisOutput {
    pub facts: Vec<LocatedFact>,
    pub issues: Vec<AnalysisIssue>,
}

pub trait Analyzer: Send + Sync {
    fn id(&self) -> &'static str;
    fn analyze(&self, tree: &FileTree) -> AnalysisOutput;
}

pub fn builtin_analyzers() -> Vec<Box<dyn Analyzer>> {
    vec![
        Box::new(migration::MigrationAnalyzer),
        Box::new(application::ApplicationAnalyzer),
        Box::new(deployment::DeploymentAnalyzer),
        Box::new(configuration::ConfigurationAnalyzer),
    ]
}
