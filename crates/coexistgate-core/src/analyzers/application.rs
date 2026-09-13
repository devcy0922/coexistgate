use regex::Regex;
use std::sync::OnceLock;

use crate::analyzers::{AnalysisOutput, Analyzer};
use crate::discovery::{files_of, ArtifactKind};
use crate::fact::{Fact, LocatedFact};
use crate::tree::FileTree;

pub struct ApplicationAnalyzer;

impl Analyzer for ApplicationAnalyzer {
    fn id(&self) -> &'static str {
        "application"
    }

    fn analyze(&self, tree: &FileTree) -> AnalysisOutput {
        let mut out = Vec::new();
        for file in files_of(tree, ArtifactKind::Application) {
            out.extend(extract(&file.path, &file.content));
        }
        out.sort_by(|a, b| {
            (&a.path, a.line, fact_key(&a.fact)).cmp(&(&b.path, b.line, fact_key(&b.fact)))
        });
        AnalysisOutput {
            facts: out,
            issues: Vec::new(),
        }
    }
}

fn fact_key(f: &Fact) -> String {
    match f {
        Fact::ColumnReference { table, column } => format!("c:{table}.{column}"),
        Fact::TableReference { table } => format!("t:{table}"),
        Fact::EnvReference { key } => format!("e:{key}"),
        _ => String::new(),
    }
}

fn extract(path: &str, content: &str) -> Vec<LocatedFact> {
    let uncommented = strip_js_comments(content);
    let mut facts = Vec::new();
    facts.extend(column_refs(path, &uncommented));
    facts.extend(sql_from_strings(path, &uncommented));
    facts.extend(env_refs(path, &uncommented));
    facts
}

/// Preserve newlines so line numbers stay aligned; remove // and /* */ comments.
fn strip_js_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    let mut in_s = false;
    let mut in_d = false;
    let mut in_t = false;
    let mut in_line = false;
    let mut in_block = false;
    while let Some(c) = chars.next() {
        if in_line {
            if c == '\n' {
                in_line = false;
                out.push('\n');
            }
            continue;
        }
        if in_block {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block = false;
            } else if c == '\n' {
                out.push('\n');
            }
            continue;
        }
        if c == '\\' && (in_s || in_d || in_t) {
            out.push(c);
            if let Some(n) = chars.next() {
                out.push(n);
            }
            continue;
        }
        if !in_s && !in_d && !in_t {
            if c == '/' && chars.peek() == Some(&'/') {
                chars.next();
                in_line = true;
                continue;
            }
            if c == '/' && chars.peek() == Some(&'*') {
                chars.next();
                in_block = true;
                continue;
            }
        }
        if c == '\'' && !in_d && !in_t {
            in_s = !in_s;
        } else if c == '"' && !in_s && !in_t {
            in_d = !in_d;
        } else if c == '`' && !in_s && !in_d {
            in_t = !in_t;
        }
        out.push(c);
    }
    out
}

fn column_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\b([A-Za-z_][A-Za-z0-9_]*)\.([A-Za-z_][A-Za-z0-9_]*)\b").unwrap()
    })
}

fn env_dot_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\b(?:process\.env|Deno\.env|import\.meta\.env)\.([A-Z][A-Z0-9_]+)\b").unwrap()
    })
}

fn env_index_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"\b(?:process\.env|Deno\.env\.get|import\.meta\.env)\s*(?:\(|\[)\s*['"]([A-Z][A-Z0-9_]+)['"]"#).unwrap()
    })
}

fn sql_from_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\b(?:FROM|INTO|UPDATE|JOIN)\s+([A-Za-z_][A-Za-z0-9_]*)\b").unwrap()
    })
}

const JS_STOP: &[&str] = &[
    "this",
    "console",
    "window",
    "document",
    "module",
    "exports",
    "require",
    "process",
    "Math",
    "Object",
    "Array",
    "String",
    "Number",
    "Boolean",
    "JSON",
    "Error",
    "Promise",
    "Date",
    "Map",
    "Set",
    "Buffer",
    "global",
    "globalThis",
    "import",
    "meta",
    "Deno",
    "props",
    "state",
    "ctx",
    "req",
    "res",
    "app",
    "router",
    "schema",
    "prisma",
    "knex",
    "client",
    "db",
    "sql",
    "query",
    "result",
    "row",
    "rows",
    "config",
    "env",
];

fn is_table_name(name: &str) -> bool {
    let l = name.to_ascii_lowercase();
    if matches!(
        l.as_str(),
        "this"
            | "console"
            | "window"
            | "document"
            | "module"
            | "process"
            | "math"
            | "object"
            | "array"
            | "string"
            | "number"
            | "json"
            | "error"
            | "promise"
            | "date"
            | "map"
            | "set"
            | "buffer"
            | "global"
            | "import"
            | "meta"
            | "deno"
            | "props"
            | "state"
            | "length"
            | "prototype"
            | "constructor"
            | "tostring"
            | "env"
            | "log"
            | "info"
            | "warn"
            | "stdout"
            | "stderr"
            | "config"
    ) {
        return false;
    }
    // Qualified SQL-style table.column: table names in this demo language are lowercase identifiers.
    name.chars().all(|c| c.is_ascii_lowercase() || c == '_') && name.len() >= 2
}

fn is_column_name(name: &str) -> bool {
    !matches!(
        name,
        "length"
            | "map"
            | "filter"
            | "then"
            | "catch"
            | "finally"
            | "toString"
            | "valueOf"
            | "prototype"
            | "constructor"
            | "name"
            | "stack"
            | "message"
            | "env"
            | "argv"
            | "exit"
            | "cwd"
            | "log"
            | "error"
            | "info"
            | "warn"
            | "debug"
    )
}

fn column_refs(path: &str, src: &str) -> Vec<LocatedFact> {
    let mut out = Vec::new();
    for (i, line) in src.lines().enumerate() {
        for cap in column_re().captures_iter(line) {
            let table = cap[1].to_string();
            let column = cap[2].to_string();
            if JS_STOP.contains(&table.as_str()) {
                continue;
            }
            if !is_table_name(&table) || !is_column_name(&column) {
                continue;
            }
            out.push(LocatedFact {
                path: path.to_string(),
                line: (i as u32) + 1,
                fact: Fact::ColumnReference {
                    table: table.to_ascii_lowercase(),
                    column: column.to_ascii_lowercase(),
                },
            });
            out.push(LocatedFact {
                path: path.to_string(),
                line: (i as u32) + 1,
                fact: Fact::TableReference {
                    table: table.to_ascii_lowercase(),
                },
            });
        }
    }
    out
}

fn sql_from_strings(path: &str, src: &str) -> Vec<LocatedFact> {
    let mut out = Vec::new();
    for (i, line) in src.lines().enumerate() {
        for cap in sql_from_re().captures_iter(line) {
            let table = ident_norm(&cap[1]);
            if table.len() < 2 {
                continue;
            }
            out.push(LocatedFact {
                path: path.to_string(),
                line: (i as u32) + 1,
                fact: Fact::TableReference { table },
            });
        }
    }
    out
}

fn ident_norm(s: &str) -> String {
    s.trim_matches('"').to_ascii_lowercase()
}

fn env_refs(path: &str, src: &str) -> Vec<LocatedFact> {
    let mut out = Vec::new();
    for (i, line) in src.lines().enumerate() {
        for cap in env_dot_re().captures_iter(line) {
            out.push(LocatedFact {
                path: path.to_string(),
                line: (i as u32) + 1,
                fact: Fact::EnvReference {
                    key: cap[1].to_string(),
                },
            });
        }
        for cap in env_index_re().captures_iter(line) {
            out.push(LocatedFact {
                path: path.to_string(),
                line: (i as u32) + 1,
                fact: Fact::EnvReference {
                    key: cap[1].to_string(),
                },
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_users_email_and_env() {
        let mut t = FileTree::new();
        t.insert(
            "src/user_repository.ts",
            r#"
export function find(db: Db) {
  return db.query(`SELECT users.email FROM users`);
}
const url = process.env.REDIS_URL;
"#,
        )
        .unwrap();
        let facts = ApplicationAnalyzer.analyze(&t).facts;
        assert!(facts.iter().any(|f| matches!(
            &f.fact,
            Fact::ColumnReference { table, column } if table == "users" && column == "email"
        )));
        assert!(facts
            .iter()
            .any(|f| matches!(&f.fact, Fact::EnvReference { key } if key == "REDIS_URL")));
    }

    #[test]
    fn ignores_commented_refs() {
        let mut t = FileTree::new();
        t.insert("src/a.ts", "// users.email\nconst x = 1;\n")
            .unwrap();
        let facts = ApplicationAnalyzer.analyze(&t).facts;
        assert!(facts
            .iter()
            .all(|f| !matches!(f.fact, Fact::ColumnReference { .. })));
    }
}
