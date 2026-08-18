use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum WebhookError {
    #[error("not a pull_request event")]
    NotPullRequest,
    #[error("ignored action: {0}")]
    IgnoredAction(String),
    #[error("invalid json: {0}")]
    Json(String),
    #[error("missing field: {0}")]
    Missing(&'static str),
}

#[derive(Clone, Debug)]
pub struct PullRequestAnalysis {
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub base_sha: String,
    pub head_sha: String,
    pub installation_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct Event {
    action: Option<String>,
    pull_request: Option<PullRequest>,
    repository: Option<Repository>,
    installation: Option<Installation>,
}

#[derive(Debug, Deserialize)]
struct PullRequest {
    number: u64,
    base: Option<RefSha>,
    head: Option<RefSha>,
}

#[derive(Debug, Deserialize)]
struct RefSha {
    sha: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Repository {
    name: Option<String>,
    owner: Option<Owner>,
}

#[derive(Debug, Deserialize)]
struct Owner {
    login: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Installation {
    id: Option<u64>,
}

const HANDLE: &[&str] = &["opened", "synchronize", "reopened", "ready_for_review"];

pub fn parse_pull_request_event(event_name: &str, body: &[u8]) -> Result<PullRequestAnalysis, WebhookError> {
    if event_name != "pull_request" {
        return Err(WebhookError::NotPullRequest);
    }
    let ev: Event = serde_json::from_slice(body).map_err(|e| WebhookError::Json(e.to_string()))?;
    let action = ev.action.unwrap_or_default();
    if !HANDLE.contains(&action.as_str()) {
        return Err(WebhookError::IgnoredAction(action));
    }
    let pr = ev.pull_request.ok_or(WebhookError::Missing("pull_request"))?;
    let repo = ev.repository.ok_or(WebhookError::Missing("repository"))?;
    Ok(PullRequestAnalysis {
        owner: repo
            .owner
            .and_then(|o| o.login)
            .ok_or(WebhookError::Missing("owner"))?,
        repo: repo.name.ok_or(WebhookError::Missing("name"))?,
        number: pr.number,
        base_sha: pr
            .base
            .and_then(|b| b.sha)
            .ok_or(WebhookError::Missing("base.sha"))?,
        head_sha: pr
            .head
            .and_then(|h| h.sha)
            .ok_or(WebhookError::Missing("head.sha"))?,
        installation_id: ev.installation.and_then(|i| i.id),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_opened_pr() {
        let body = br#"{
          "action": "opened",
          "pull_request": {
            "number": 7,
            "base": {"sha": "aaa"},
            "head": {"sha": "bbb"}
          },
          "repository": {"name": "demo", "owner": {"login": "acme"}},
          "installation": {"id": 99}
        }"#;
        let p = parse_pull_request_event("pull_request", body).unwrap();
        assert_eq!(p.number, 7);
        assert_eq!(p.owner, "acme");
        assert_eq!(p.installation_id, Some(99));
    }

    #[test]
    fn ignores_closed() {
        let body = br#"{"action":"closed","pull_request":{"number":1,"base":{"sha":"a"},"head":{"sha":"b"}},"repository":{"name":"d","owner":{"login":"o"}}}"#;
        let err = parse_pull_request_event("pull_request", body).unwrap_err();
        assert!(matches!(err, WebhookError::IgnoredAction(_)));
    }
}
