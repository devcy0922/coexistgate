use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::Error;

/// Deterministic snapshot of a repository tree (previous or candidate).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileTree {
    files: BTreeMap<String, SourceFile>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFile {
    pub path: String,
    pub content: String,
}

impl FileTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(
        &mut self,
        path: impl Into<String>,
        content: impl Into<String>,
    ) -> crate::Result<()> {
        let path = normalize_path(&path.into())?;
        self.files.insert(
            path.clone(),
            SourceFile {
                path,
                content: content.into(),
            },
        );
        Ok(())
    }

    pub fn get(&self, path: &str) -> Option<&SourceFile> {
        self.files.get(path)
    }

    pub fn iter(&self) -> impl Iterator<Item = &SourceFile> {
        self.files.values()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(|s| s.as_str())
    }
}

pub fn normalize_path(path: &str) -> crate::Result<String> {
    let path = path.replace('\\', "/");
    if path.is_empty() {
        return Err(Error::Path("empty path".into()));
    }
    if Path::new(&path)
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(Error::Path(format!("path traversal rejected: {path}")));
    }
    let mut out = PathBuf::new();
    for c in Path::new(&path).components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::Prefix(_) | std::path::Component::RootDir => {}
            other => out.push(other),
        }
    }
    Ok(out.to_string_lossy().replace('\\', "/"))
}
