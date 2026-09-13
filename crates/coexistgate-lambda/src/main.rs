use coexistgate_github::{
    create_check_run, fetch_tree, installation_token, parse_pull_request_event, verify_signature,
    GitHubConfig, WebhookError,
};
use lambda_http::{
    http::StatusCode, run, service_fn, Body, Error as LambdaError, Request, Response,
};

const MAX_BODY: usize = 1024 * 1024;

#[tokio::main]
async fn main() -> Result<(), LambdaError> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .without_time()
        .init();
    run(service_fn(handler)).await
}

async fn handler(req: Request) -> Result<Response<Body>, LambdaError> {
    let request_id = std::env::var("AWS_REQUEST_ID").unwrap_or_else(|_| "local".into());
    let span = tracing::info_span!("webhook", request_id = %request_id);
    let _g = span.enter();

    if req.method() != lambda_http::http::Method::POST {
        return ok(StatusCode::METHOD_NOT_ALLOWED, "POST only");
    }

    let body = match req.body() {
        Body::Text(s) => s.as_bytes().to_vec(),
        Body::Binary(b) => b.clone(),
        Body::Empty => Vec::new(),
    };
    if body.len() > MAX_BODY {
        tracing::warn!(size = body.len(), "request too large");
        return ok(StatusCode::PAYLOAD_TOO_LARGE, "payload too large");
    }

    let cfg = match GitHubConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "config");
            return ok(StatusCode::INTERNAL_SERVER_ERROR, "misconfigured");
        }
    };

    let sig = header(&req, "x-hub-signature-256");
    if !verify_signature(cfg.webhook_secret.as_bytes(), &body, &sig) {
        tracing::warn!("invalid webhook signature");
        return ok(StatusCode::UNAUTHORIZED, "invalid signature");
    }

    let event = header(&req, "x-github-event");
    if event == "ping" {
        return ok(StatusCode::OK, "pong");
    }

    let pr = match parse_pull_request_event(&event, &body) {
        Ok(p) => p,
        Err(WebhookError::NotPullRequest) | Err(WebhookError::IgnoredAction(_)) => {
            tracing::info!(event = %event, "ignored");
            return ok(StatusCode::OK, "ignored");
        }
        Err(e) => {
            tracing::warn!(error = %e, "parse");
            return ok(StatusCode::BAD_REQUEST, "bad event");
        }
    };

    let Some(install) = pr.installation_id else {
        tracing::warn!("missing installation id");
        return ok(StatusCode::BAD_REQUEST, "missing installation");
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(40))
        .build()?;

    let token = match installation_token(&client, &cfg, install).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "token");
            return ok(StatusCode::BAD_GATEWAY, "github auth failed");
        }
    };

    let previous = match fetch_tree(
        &client,
        &token,
        &cfg.api_base,
        &pr.owner,
        &pr.repo,
        &pr.base_sha,
    )
    .await
    {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "base tree");
            return ok(StatusCode::BAD_GATEWAY, "fetch base failed");
        }
    };
    let candidate = match fetch_tree(
        &client,
        &token,
        &cfg.api_base,
        &pr.owner,
        &pr.repo,
        &pr.head_sha,
    )
    .await
    {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "head tree");
            return ok(StatusCode::BAD_GATEWAY, "fetch head failed");
        }
    };

    let report = match coexistgate_github::analyze_pull_request_trees(previous, candidate) {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "analyze");
            return ok(StatusCode::INTERNAL_SERVER_ERROR, "analyze failed");
        }
    };

    tracing::info!(
        gate = ?report.gate,
        findings = report.findings.len(),
        critical = report.categories.critical,
        high = report.categories.high,
        "gate"
    );

    if let Err(e) = create_check_run(
        &client,
        &token,
        &cfg.api_base,
        &pr.owner,
        &pr.repo,
        &pr.head_sha,
        &report,
    )
    .await
    {
        tracing::error!(error = %e, "check run");
        return ok(StatusCode::BAD_GATEWAY, "check run failed");
    }

    ok(StatusCode::OK, "ok")
}

fn header(req: &Request, name: &str) -> String {
    req.headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string()
}

fn ok(status: StatusCode, msg: &'static str) -> Result<Response<Body>, LambdaError> {
    Ok(Response::builder()
        .status(status)
        .header("content-type", "text/plain")
        .body(Body::from(msg))?)
}
