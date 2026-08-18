use crate::analyzers::Analyzer;
use crate::discovery::{files_of, ArtifactKind};
use crate::fact::{EnvSource, Fact, LocatedFact};
use crate::tree::FileTree;

pub struct ConfigurationAnalyzer;

impl Analyzer for ConfigurationAnalyzer {
    fn id(&self) -> &'static str {
        "configuration"
    }

    fn analyze(&self, tree: &FileTree) -> Vec<LocatedFact> {
        let mut out = Vec::new();
        for file in files_of(tree, ArtifactKind::Configuration) {
            out.extend(parse_dotenv(&file.path, &file.content));
        }
        out.sort_by(|a, b| (&a.path, a.line, env_key(&a.fact)).cmp(&(&b.path, b.line, env_key(&b.fact))));
        out
    }
}

fn env_key(f: &Fact) -> &str {
    match f {
        Fact::EnvDefinition { key, .. } => key,
        _ => "",
    }
}

fn parse_dotenv(path: &str, content: &str) -> Vec<LocatedFact> {
    let mut out = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let rest = trimmed.strip_prefix("export ").unwrap_or(trimmed);
        let Some((key, _)) = rest.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() || key.starts_with('#') {
            continue;
        }
        out.push(LocatedFact {
            path: path.to_string(),
            line: (i as u32) + 1,
            fact: Fact::EnvDefinition {
                key: key.to_string(),
                source: EnvSource::DotEnv,
            },
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_env_example() {
        let mut t = FileTree::new();
        t.insert(".env.example", "REDIS_URL=redis://localhost\n# secret\nCACHE_URL=\n")
            .unwrap();
        let facts = ConfigurationAnalyzer.analyze(&t);
        let keys: Vec<_> = facts
            .iter()
            .filter_map(|f| match &f.fact {
                Fact::EnvDefinition { key, .. } => Some(key.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(keys, ["REDIS_URL", "CACHE_URL"]);
    }
}
