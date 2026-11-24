use crate::settings::Config;
use anyhow::{Context, Result, bail};
use reqwest::{Client, StatusCode, header};
use serde::Deserialize;
use std::time::Duration;
use url::Url;

#[derive(Debug, Clone)]
pub struct Issue {
    pub repository: String,
    pub number: u64,
    pub title: String,
    pub state: String,
    pub author: String,
    pub body: Option<String>,
    pub labels: Vec<String>,
    pub comments: Vec<Comment>,
    pub web_url: String,
}

#[derive(Debug, Clone)]
pub struct Comment {
    pub author: Option<String>,
    pub body: Option<String>,
}

#[derive(Deserialize)]
struct ApiIssue {
    number: u64,
    title: String,
    state: String,
    body: Option<String>,
    user: ApiUser,
    labels: Vec<ApiLabel>,
}
#[derive(Deserialize)]
struct ApiUser {
    login: String,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum ApiLabel {
    Named { name: String },
    Other(String),
}
#[derive(Deserialize)]
struct ApiComment {
    user: Option<ApiUser>,
    body: Option<String>,
}

pub async fn fetch(raw_url: &str, config: &Config) -> Result<Issue> {
    let url = Url::parse(raw_url).context("invalid issue URL")?;
    if url.scheme() != "https" {
        bail!("only HTTPS issue URLs are accepted");
    }
    if !url.username().is_empty() || url.password().is_some() || url.port().is_some() {
        bail!("issue URL must not contain credentials or a custom port");
    }
    if url.host_str() != Some(config.providers.github_web_host.as_str()) {
        bail!("issue host is not configured as the GitHub web host");
    }
    let parts: Vec<_> = url
        .path_segments()
        .context("issue URL has no repository path")?
        .collect();
    if parts.len() != 4 || !matches!(parts[2], "issues" | "pull") {
        bail!("expected a GitHub issue URL with owner, repository, and issue number");
    }
    let owner = parts[0];
    let repo = parts[1];
    let number: u64 = parts[3].parse().context("invalid issue number")?;
    if owner.is_empty() || repo.is_empty() || number == 0 {
        bail!("incomplete issue URL");
    }
    if !owner.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        || !repo
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        bail!("issue URL contains an invalid owner or repository name");
    }
    let repository = format!("{owner}/{repo}");
    let api = format!(
        "{}/repos/{repository}/issues/{number}",
        config.providers.github_api_base.trim_end_matches('/')
    );
    let client = Client::builder()
        .timeout(Duration::from_secs(
            config.providers.request_timeout_seconds,
        ))
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("ali/", env!("CARGO_PKG_VERSION")))
        .default_headers({
            let mut h = header::HeaderMap::new();
            h.insert(
                header::ACCEPT,
                header::HeaderValue::from_static("application/vnd.github+json"),
            );
            h
        })
        .build()?;
    let request = client.get(&api);
    let response = request
        .send()
        .await
        .context("network error while fetching issue")?;
    if response.status() == StatusCode::NOT_FOUND {
        bail!("issue not found or repository is private; only public GitHub issues are supported");
    }
    if response.status() == StatusCode::TOO_MANY_REQUESTS {
        bail!("GitHub API rate limit reached; retry after the configured service window");
    }
    let raw: ApiIssue = response
        .error_for_status()
        .context("GitHub API rejected the issue request")?
        .json()
        .await
        .context("invalid GitHub issue response")?;
    let comments_url = format!("{api}/comments?per_page={}", config.providers.max_comments);
    let request = client.get(comments_url);
    let comments: Vec<ApiComment> = request
        .send()
        .await
        .context("network error while fetching comments")?
        .error_for_status()
        .context("GitHub API rejected the comments request")?
        .json()
        .await
        .context("invalid GitHub comments response")?;
    Ok(Issue {
        repository,
        number: raw.number,
        title: raw.title,
        state: raw.state,
        author: raw.user.login,
        body: raw.body,
        labels: raw
            .labels
            .into_iter()
            .map(|l| match l {
                ApiLabel::Named { name } => name,
                ApiLabel::Other(s) => s,
            })
            .collect(),
        comments: comments
            .into_iter()
            .map(|c| Comment {
                author: c.user.map(|user| user.login),
                body: c.body,
            })
            .collect(),
        web_url: url.to_string(),
    })
}
