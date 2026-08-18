use regex::Regex;

use crate::analyzers::Analyzer;
use crate::discovery::{files_of, ArtifactKind};
use crate::fact::{Fact, LocatedFact, SchemaOp};
use crate::tree::FileTree;

pub struct MigrationAnalyzer;

impl Analyzer for MigrationAnalyzer {
    fn id(&self) -> &'static str {
        "migration"
    }

    fn analyze(&self, tree: &FileTree) -> Vec<LocatedFact> {
        let mut out = Vec::new();
        for file in files_of(tree, ArtifactKind::Migration) {
            out.extend(extract_sql(&file.path, &file.content));
        }
        out.sort_by(|a, b| (&a.path, a.line).cmp(&(&b.path, b.line)));
        out
    }
}

fn extract_sql(path: &str, content: &str) -> Vec<LocatedFact> {
    let mut facts = Vec::new();
    for stmt in split_statements(content) {
        if let Some(fact) = parse_statement(&stmt.text) {
            facts.push(LocatedFact {
                path: path.to_string(),
                line: stmt.line,
                fact,
            });
        }
    }
    facts
}

struct Stmt {
    line: u32,
    text: String,
}

fn split_statements(content: &str) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    let mut buf = String::new();
    let mut start_line = 1u32;
    let mut line = 1u32;
    let mut chars = content.chars().peekable();
    let mut in_squote = false;
    let mut in_dquote = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    while let Some(c) = chars.next() {
        if c == '\n' {
            line += 1;
        }
        if in_line_comment {
            if c == '\n' {
                in_line_comment = false;
            }
            continue;
        }
        if in_block_comment {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block_comment = false;
            }
            continue;
        }
        if !in_squote && !in_dquote {
            if c == '-' && chars.peek() == Some(&'-') {
                chars.next();
                in_line_comment = true;
                continue;
            }
            if c == '/' && chars.peek() == Some(&'*') {
                chars.next();
                in_block_comment = true;
                continue;
            }
        }
        if c == '\'' && !in_dquote {
            in_squote = !in_squote;
            buf.push(c);
            continue;
        }
        if c == '"' && !in_squote {
            in_dquote = !in_dquote;
            buf.push(c);
            continue;
        }
        if c == ';' && !in_squote && !in_dquote {
            let text = buf.trim().to_string();
            if !text.is_empty() {
                stmts.push(Stmt {
                    line: start_line,
                    text,
                });
            }
            buf.clear();
            start_line = line;
            continue;
        }
        if buf.is_empty() && c.is_whitespace() {
            if c == '\n' {
                start_line = line;
            }
            continue;
        }
        buf.push(c);
    }
    let text = buf.trim().to_string();
    if !text.is_empty() {
        stmts.push(Stmt {
            line: start_line,
            text,
        });
    }
    stmts
}

fn parse_statement(sql: &str) -> Option<Fact> {
    let compact = collapse_ws(sql);
    drop_table(&compact)
        .or_else(|| rename_table(&compact))
        .or_else(|| rename_column(&compact))
        .or_else(|| drop_column(&compact))
        .or_else(|| set_not_null(&compact))
        .or_else(|| add_not_null_column(&compact))
        .or_else(|| type_change(&compact))
}

fn collapse_ws(s: &str) -> String {
    let mut out = String::new();
    let mut last_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !last_space && !out.is_empty() {
                out.push(' ');
            }
            last_space = true;
        } else {
            last_space = false;
            out.push(c);
        }
    }
    out
}

fn ident(s: &str) -> String {
    s.trim()
        .trim_matches('"')
        .trim_matches('`')
        .split('.')
        .next_back()
        .unwrap_or(s)
        .to_ascii_lowercase()
}

fn drop_table(sql: &str) -> Option<Fact> {
    let r = Regex::new(r"(?i)^DROP\s+TABLE\s+(?:IF\s+EXISTS\s+)?([A-Za-z0-9_.]+)").ok()?;
    let c = r.captures(sql)?;
    let table = ident(&c[1]);
    Some(Fact::SchemaChange {
        table: table.clone(),
        operation: SchemaOp::DropTable,
        from: Some(table),
        to: None,
        type_from: None,
        type_to: None,
        raw: sql.to_string(),
    })
}

fn rename_table(sql: &str) -> Option<Fact> {
    let r = Regex::new(
        r"(?i)^ALTER\s+TABLE\s+(?:IF\s+EXISTS\s+)?([A-Za-z0-9_.]+)\s+RENAME\s+TO\s+([A-Za-z0-9_.]+)",
    )
    .ok()?;
    let c = r.captures(sql)?;
    Some(Fact::SchemaChange {
        table: ident(&c[1]),
        operation: SchemaOp::RenameTable,
        from: Some(ident(&c[1])),
        to: Some(ident(&c[2])),
        type_from: None,
        type_to: None,
        raw: sql.to_string(),
    })
}

fn rename_column(sql: &str) -> Option<Fact> {
    let r = Regex::new(
        r"(?i)^ALTER\s+TABLE\s+(?:IF\s+EXISTS\s+)?([A-Za-z0-9_.]+)\s+RENAME\s+COLUMN\s+(?:IF\s+EXISTS\s+)?([A-Za-z0-9_]+)\s+TO\s+([A-Za-z0-9_]+)",
    )
    .ok()?;
    let c = r.captures(sql)?;
    Some(Fact::SchemaChange {
        table: ident(&c[1]),
        operation: SchemaOp::RenameColumn,
        from: Some(ident(&c[2])),
        to: Some(ident(&c[3])),
        type_from: None,
        type_to: None,
        raw: sql.to_string(),
    })
}

fn drop_column(sql: &str) -> Option<Fact> {
    let r = Regex::new(
        r"(?i)^ALTER\s+TABLE\s+(?:IF\s+EXISTS\s+)?([A-Za-z0-9_.]+)\s+DROP\s+COLUMN\s+(?:IF\s+EXISTS\s+)?([A-Za-z0-9_]+)",
    )
    .ok()?;
    let c = r.captures(sql)?;
    Some(Fact::SchemaChange {
        table: ident(&c[1]),
        operation: SchemaOp::DropColumn,
        from: Some(ident(&c[2])),
        to: None,
        type_from: None,
        type_to: None,
        raw: sql.to_string(),
    })
}

fn set_not_null(sql: &str) -> Option<Fact> {
    let r = Regex::new(
        r"(?i)^ALTER\s+TABLE\s+(?:IF\s+EXISTS\s+)?([A-Za-z0-9_.]+)\s+ALTER\s+COLUMN\s+([A-Za-z0-9_]+)\s+SET\s+NOT\s+NULL",
    )
    .ok()?;
    let c = r.captures(sql)?;
    Some(Fact::SchemaChange {
        table: ident(&c[1]),
        operation: SchemaOp::SetNotNull,
        from: Some(ident(&c[2])),
        to: None,
        type_from: None,
        type_to: None,
        raw: sql.to_string(),
    })
}

fn add_not_null_column(sql: &str) -> Option<Fact> {
    let r = Regex::new(
        r"(?i)^ALTER\s+TABLE\s+(?:IF\s+EXISTS\s+)?([A-Za-z0-9_.]+)\s+ADD\s+COLUMN\s+(?:IF\s+NOT\s+EXISTS\s+)?([A-Za-z0-9_]+)\s+([A-Za-z0-9_()]+)(.*)$",
    )
    .ok()?;
    let c = r.captures(sql)?;
    let rest = c[4].to_ascii_uppercase();
    let has_not_null = rest.contains("NOT NULL");
    let has_default = rest.contains("DEFAULT");
    if has_not_null && !has_default {
        return Some(Fact::SchemaChange {
            table: ident(&c[1]),
            operation: SchemaOp::AddNotNullColumn,
            from: None,
            to: Some(ident(&c[2])),
            type_from: None,
            type_to: Some(c[3].to_ascii_lowercase()),
            raw: sql.to_string(),
        });
    }
    None
}

fn type_change(sql: &str) -> Option<Fact> {
    let r = Regex::new(
        r"(?i)^ALTER\s+TABLE\s+(?:IF\s+EXISTS\s+)?([A-Za-z0-9_.]+)\s+ALTER\s+COLUMN\s+([A-Za-z0-9_]+)\s+TYPE\s+([A-Za-z0-9_()]+)",
    )
    .ok()?;
    let c = r.captures(sql)?;
    Some(Fact::SchemaChange {
        table: ident(&c[1]),
        operation: SchemaOp::TypeChange,
        from: Some(ident(&c[2])),
        to: Some(ident(&c[2])),
        type_from: None,
        type_to: Some(c[3].to_ascii_lowercase()),
        raw: sql.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::FileTree;

    #[test]
    fn extracts_rename_column() {
        let mut t = FileTree::new();
        t.insert(
            "migrations/002.sql",
            "ALTER TABLE users RENAME COLUMN email TO email_address;\n",
        )
        .unwrap();
        let facts = MigrationAnalyzer.analyze(&t);
        assert_eq!(facts.len(), 1);
        match &facts[0].fact {
            Fact::SchemaChange {
                table,
                operation,
                from,
                to,
                ..
            } => {
                assert_eq!(table, "users");
                assert_eq!(*operation, SchemaOp::RenameColumn);
                assert_eq!(from.as_deref(), Some("email"));
                assert_eq!(to.as_deref(), Some("email_address"));
            }
            _ => panic!("wrong fact"),
        }
    }

    #[test]
    fn ignores_comments() {
        let mut t = FileTree::new();
        t.insert(
            "migrations/001.sql",
            "-- ALTER TABLE users DROP COLUMN email;\nSELECT 1;\n",
        )
        .unwrap();
        assert!(MigrationAnalyzer.analyze(&t).is_empty());
    }
}
