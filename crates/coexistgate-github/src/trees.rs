use std::io::{Cursor, Read};

use coexistgate_core::FileTree;
use zip::ZipArchive;

use crate::Error;

#[derive(Clone, Copy, Debug)]
pub struct ZipLimits {
    pub max_compressed: u64,
    pub max_uncompressed: u64,
    pub max_files: usize,
    pub max_file_bytes: u64,
}

impl Default for ZipLimits {
    fn default() -> Self {
        Self {
            max_compressed: 20 * 1024 * 1024,
            max_uncompressed: 50 * 1024 * 1024,
            max_files: 4000,
            max_file_bytes: 1024 * 1024,
        }
    }
}

pub async fn download_zipball(
    client: &reqwest::Client,
    token: &str,
    api_base: &str,
    owner: &str,
    repo: &str,
    sha: &str,
) -> Result<FileTree, Error> {
    let url = format!(
        "{}/repos/{owner}/{repo}/zipball/{sha}",
        api_base.trim_end_matches('/')
    );
    let resp = client
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "coexistgate")
        .send()
        .await
        .map_err(|e| Error::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(Error::Http(format!("zipball {}", resp.status())));
    }
    let bytes = resp.bytes().await.map_err(|e| Error::Http(e.to_string()))?;
    tree_from_zip(&bytes, ZipLimits::default())
}

pub fn tree_from_zip(bytes: &[u8], limits: ZipLimits) -> Result<FileTree, Error> {
    if bytes.len() as u64 > limits.max_compressed {
        return Err(Error::Zip("compressed archive too large".into()));
    }
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(|e| Error::Zip(e.to_string()))?;
    if zip.len() > limits.max_files {
        return Err(Error::Zip("too many files".into()));
    }
    let mut tree = FileTree::new();
    let mut uncompressed = 0u64;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(|e| Error::Zip(e.to_string()))?;
        if !file.is_file() {
            continue;
        }
        let name = file.name().replace('\\', "/");
        if name.contains("..") {
            return Err(Error::Zip(format!("path traversal: {name}")));
        }
        let rel = strip_zip_prefix(&name);
        if skip(rel) {
            continue;
        }
        let size = file.size();
        if size > limits.max_file_bytes {
            continue;
        }
        uncompressed = uncompressed.saturating_add(size);
        if uncompressed > limits.max_uncompressed {
            return Err(Error::Zip("uncompressed archive too large".into()));
        }
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|e| Error::Zip(e.to_string()))?;
        if let Ok(text) = String::from_utf8(buf) {
            tree.insert(rel, text).map_err(|e| Error::Zip(e.to_string()))?;
        }
    }
    Ok(tree)
}

fn strip_zip_prefix(name: &str) -> &str {
    match name.split_once('/') {
        Some((_, rest)) => rest,
        None => name,
    }
}

fn skip(rel: &str) -> bool {
    rel.split('/').any(|seg| {
        matches!(
            seg,
            ".git" | "node_modules" | "target" | "dist" | "vendor" | ".next"
        )
    }) || rel.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    #[test]
    fn rejects_zip_slip() {
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(Cursor::new(&mut buf));
            w.start_file("../etc/passwd", SimpleFileOptions::default())
                .unwrap();
            w.write_all(b"nope").unwrap();
            w.finish().unwrap();
        }
        let err = tree_from_zip(&buf, ZipLimits::default()).unwrap_err();
        assert!(err.to_string().contains("traversal"));
    }

    #[test]
    fn reads_nested_file() {
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(Cursor::new(&mut buf));
            w.start_file("repo-sha/src/app.ts", SimpleFileOptions::default())
                .unwrap();
            w.write_all(b"const x = 1;\n").unwrap();
            w.finish().unwrap();
        }
        let tree = tree_from_zip(&buf, ZipLimits::default()).unwrap();
        assert_eq!(tree.get("src/app.ts").unwrap().content, "const x = 1;\n");
    }
}
