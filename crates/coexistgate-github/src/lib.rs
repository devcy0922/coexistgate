//! GitHub adapter. Core stays unaware of GitHub types.

mod check;
mod signature;
mod trees;
mod webhook;

pub use check::{check_run_body, CheckConclusion};
pub use signature::{signature_header, verify_signature};
pub use trees::{tree_from_zip, ZipLimits};
pub use webhook::{parse_pull_request_event, PullRequestAnalysis, WebhookError};

use coexistgate_core::{analyze, AnalysisRequest, FileTree, Report};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("webhook: {0}")]
    Webhook(#[from] WebhookError),
    #[error("http: {0}")]
    Http(String),
    #[error("zip: {0}")]
    Zip(String),
    #[error("core: {0}")]
    Core(#[from] coexistgate_core::Error),
    #[error("auth: {0}")]
    Auth(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug)]
pub struct GitHubConfig {
    pub app_id: u64,
    pub private_key_pem: String,
    pub webhook_secret: String,
    pub api_base: String,
}

impl GitHubConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            app_id: std::env::var("GITHUB_APP_ID")
                .map_err(|_| Error::Auth("GITHUB_APP_ID missing".into()))?
                .parse()
                .map_err(|_| Error::Auth("GITHUB_APP_ID invalid".into()))?,
            private_key_pem: std::env::var("GITHUB_APP_PRIVATE_KEY")
                .map_err(|_| Error::Auth("GITHUB_APP_PRIVATE_KEY missing".into()))?,
            webhook_secret: std::env::var("GITHUB_WEBHOOK_SECRET")
                .map_err(|_| Error::Auth("GITHUB_WEBHOOK_SECRET missing".into()))?,
            api_base: std::env::var("GITHUB_API_BASE")
                .unwrap_or_else(|_| "https://api.github.com".into()),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdapterOutcome {
    pub ignored: bool,
    pub reason: Option<String>,
    pub report: Option<Report>,
}

/// Verify webhook, fetch base/head trees, run core, return report (caller posts the Check).
pub fn analyze_pull_request_trees(previous: FileTree, candidate: FileTree) -> Result<Report> {
    Ok(analyze(AnalysisRequest {
        previous,
        candidate,
        policy: None,
        fail_on: None,
    })?)
}

pub async fn fetch_tree(
    client: &reqwest::Client,
    token: &str,
    api_base: &str,
    owner: &str,
    repo: &str,
    sha: &str,
) -> Result<FileTree> {
    trees::download_zipball(client, token, api_base, owner, repo, sha).await
}

pub async fn installation_token(
    client: &reqwest::Client,
    cfg: &GitHubConfig,
    installation_id: u64,
) -> Result<String> {
    let jwt = app_jwt(cfg)?;
    let url = format!(
        "{}/app/installations/{installation_id}/access_tokens",
        cfg.api_base.trim_end_matches('/')
    );
    let resp = client
        .post(url)
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "coexistgate")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|e| Error::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(Error::Http(format!(
            "installation token: {}",
            resp.status()
        )));
    }
    let v: serde_json::Value = resp.json().await.map_err(|e| Error::Http(e.to_string()))?;
    v.get("token")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| Error::Auth("no token in installation response".into()))
}

pub fn app_jwt(cfg: &GitHubConfig) -> Result<String> {
    #[derive(Serialize)]
    struct Claims {
        iat: i64,
        exp: i64,
        iss: u64,
    }
    let now = chrono::Utc::now().timestamp();
    let claims = Claims {
        iat: now - 10,
        exp: now + 8 * 60,
        iss: cfg.app_id,
    };
    let key = jsonwebtoken::EncodingKey::from_rsa_pem(cfg.private_key_pem.as_bytes())
        .map_err(|e| Error::Auth(e.to_string()))?;
    let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    jsonwebtoken::encode(&header, &claims, &key).map_err(|e| Error::Auth(e.to_string()))
}

pub async fn create_check_run(
    client: &reqwest::Client,
    token: &str,
    api_base: &str,
    owner: &str,
    repo: &str,
    head_sha: &str,
    report: &Report,
) -> Result<()> {
    let url = format!(
        "{}/repos/{owner}/{repo}/check-runs",
        api_base.trim_end_matches('/')
    );
    let body = check_run_body(head_sha, report);
    let resp = client
        .post(url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "coexistgate")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .json(&body)
        .send()
        .await
        .map_err(|e| Error::Http(e.to_string()))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(Error::Http(format!("check-run {status}: {text}")));
    }
    Ok(())
}

// base64 is used in from_env optional path — keep a tiny dep via workspace? add to github crate.
