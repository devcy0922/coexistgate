use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocatedFact {
    pub path: String,
    pub line: u32,
    pub fact: Fact,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "fact", rename_all = "snake_case")]
pub enum Fact {
    SchemaChange {
        table: String,
        operation: SchemaOp,
        from: Option<String>,
        to: Option<String>,
        type_from: Option<String>,
        type_to: Option<String>,
        raw: String,
    },
    ColumnReference {
        table: String,
        column: String,
    },
    TableReference {
        table: String,
    },
    EnvReference {
        key: String,
    },
    EnvDefinition {
        key: String,
        source: EnvSource,
    },
    DeploymentStrategy {
        kind: StrategyKind,
    },
    ReplicaCount {
        count: u32,
    },
    ResourceLimit {
        cpu: Option<String>,
        memory: Option<String>,
    },
    MaxUnavailable {
        spec: String,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SchemaOp {
    DropColumn,
    RenameColumn,
    DropTable,
    RenameTable,
    SetNotNull,
    AddNotNullColumn,
    TypeChange,
}

impl SchemaOp {
    pub fn is_breaking_for_readers(self) -> bool {
        matches!(
            self,
            SchemaOp::DropColumn
                | SchemaOp::RenameColumn
                | SchemaOp::DropTable
                | SchemaOp::RenameTable
                | SchemaOp::TypeChange
        )
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SchemaOp::DropColumn => "drop_column",
            SchemaOp::RenameColumn => "rename_column",
            SchemaOp::DropTable => "drop_table",
            SchemaOp::RenameTable => "rename_table",
            SchemaOp::SetNotNull => "set_not_null",
            SchemaOp::AddNotNullColumn => "add_not_null_column",
            SchemaOp::TypeChange => "type_change",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EnvSource {
    DotEnv,
    Kubernetes,
    Compose,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StrategyKind {
    Rolling,
    Recreate,
}

