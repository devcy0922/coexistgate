use crate::tree::{FileTree, SourceFile};

const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "vendor",
    ".next",
    "coverage",
];

pub fn is_skipped_path(path: &str) -> bool {
    path.split('/')
        .any(|seg| SKIP_DIRS.contains(&seg) || seg == ".")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactKind {
    Migration,
    Application,
    Deployment,
    Configuration,
    Policy,
    Other,
}

pub fn classify(path: &str) -> ArtifactKind {
    let lower = path.to_ascii_lowercase();
    let name = lower.rsplit('/').next().unwrap_or(&lower);
    if name == ".coexistgate.yml" || name == ".coexistgate.yaml" {
        return ArtifactKind::Policy;
    }
    if name.starts_with(".env") {
        return ArtifactKind::Configuration;
    }
    if name.ends_with(".sql") {
        return ArtifactKind::Migration;
    }
    if name.contains("docker-compose") && (name.ends_with(".yml") || name.ends_with(".yaml")) {
        return ArtifactKind::Deployment;
    }
    if name.ends_with(".yml") || name.ends_with(".yaml") {
        if lower.contains("deploy") || lower.contains("k8s") || lower.contains("kube") {
            return ArtifactKind::Deployment;
        }
        if name.contains("deployment") {
            return ArtifactKind::Deployment;
        }
    }
    if matches!(
        ext(&lower).as_str(),
        "ts" | "tsx" | "js" | "jsx" | "mts" | "cts"
    ) {
        return ArtifactKind::Application;
    }
    ArtifactKind::Other
}

fn ext(path: &str) -> String {
    path.rsplit('.')
        .next()
        .filter(|s| !s.contains('/'))
        .unwrap_or("")
        .to_string()
}

pub fn files_of<'a>(tree: &'a FileTree, kind: ArtifactKind) -> Vec<&'a SourceFile> {
    tree.iter()
        .filter(|f| !is_skipped_path(&f.path) && classify(&f.path) == kind)
        .collect()
}

pub fn looks_like_helm_template(content: &str) -> bool {
    content.contains("{{") && content.contains("}}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_common_paths() {
        assert_eq!(classify("migrations/001.sql"), ArtifactKind::Migration);
        assert_eq!(classify("src/user.ts"), ArtifactKind::Application);
        assert_eq!(
            classify("deploy/deployment.yaml"),
            ArtifactKind::Deployment
        );
        assert_eq!(classify(".env.example"), ArtifactKind::Configuration);
        assert_eq!(classify(".coexistgate.yml"), ArtifactKind::Policy);
    }
}
