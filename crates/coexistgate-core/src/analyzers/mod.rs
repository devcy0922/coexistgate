mod application;
mod configuration;
mod deployment;
mod migration;

use crate::fact::LocatedFact;
use crate::tree::FileTree;

pub trait Analyzer: Send + Sync {
    fn id(&self) -> &'static str;
    fn analyze(&self, tree: &FileTree) -> Vec<LocatedFact>;
}

pub fn builtin_analyzers() -> Vec<Box<dyn Analyzer>> {
    vec![
        Box::new(migration::MigrationAnalyzer),
        Box::new(application::ApplicationAnalyzer),
        Box::new(deployment::DeploymentAnalyzer),
        Box::new(configuration::ConfigurationAnalyzer),
    ]
}

