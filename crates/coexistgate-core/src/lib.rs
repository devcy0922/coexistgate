//! CoexistGate core: artifact facts → release model → deterministic findings → gate.
//!
//! This crate must not depend on GitHub, AWS, HTTP, or LLM clients.

mod analyzers;
mod discovery;
mod engine;
mod fact;
mod finding;
mod policy;
mod release_model;
mod report;
mod rules;
mod tree;

pub use engine::{analyze, analyze_with_analyzers, AnalysisRequest};
pub use fact::{EnvSource, Fact, SchemaOp, StrategyKind};
pub use finding::{Category, Evidence, Finding, Impact, Severity};
pub use policy::{FailOn, Policy, RuleOverride};
pub use release_model::ReleaseModel;
pub use report::{render_check_summary, render_human, CategoryStatus, GateDecision, Report};
pub use rules::{rule_catalog, RuleMeta};
pub use tree::{FileTree, SourceFile};

pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PRODUCT_NAME: &str = "CoexistGate";
pub const CHECK_NAME: &str = "CoexistGate / Release Safety";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid policy: {0}")]
    Policy(String),
    #[error("invalid path: {0}")]
    Path(String),
}

pub type Result<T> = std::result::Result<T, Error>;
