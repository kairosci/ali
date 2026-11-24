use crate::settings::Config;
use anyhow::{Context, Result, bail};
use minijinja::{Environment, context};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: [Message<'a>; 2],
    temperature: f32,
}
#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}
#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}
#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}
#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

pub async fn refine(
    endpoint: &str,
    model: &str,
    brief: &str,
    config: &Config,
    locale: &str,
) -> Result<String> {
    let endpoint = endpoint.trim_end_matches('/');
    let parsed_endpoint = url::Url::parse(endpoint).context("invalid model endpoint URL")?;
    if parsed_endpoint.scheme() != "http"
        || !parsed_endpoint.username().is_empty()
        || parsed_endpoint.password().is_some()
        || parsed_endpoint.query().is_some()
        || parsed_endpoint.fragment().is_some()
        || !parsed_endpoint.host_str().is_some_and(|host| {
            host.parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
        })
    {
        bail!("model refinement is restricted to an HTTP loopback endpoint");
    }
    let path = &config.providers.openai_compatible_path;
    let url = if endpoint.ends_with(path) {
        endpoint.to_owned()
    } else {
        format!(
            "{endpoint}{}",
            if path.starts_with('/') {
                path.clone()
            } else {
                format!("/{path}")
            }
        )
    };
    let client = Client::builder()
        .timeout(Duration::from_secs(
            config.providers.request_timeout_seconds,
        ))
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let system_source = config.template(&format!("model-system/{locale}.md"))?;
    let mut templates = Environment::new();
    templates.add_template("system", &system_source)?;
    let system = templates.get_template("system")?.render(context! {})?;
    let request = ChatRequest {
        model,
        temperature: 0.2,
        messages: [
            Message {
                role: "system",
                content: &system,
            },
            Message {
                role: "user",
                content: brief,
            },
        ],
    };
    let builder = client.post(url).json(&request);
    let response: ChatResponse = builder
        .send()
        .await
        .context("model request failed")?
        .error_for_status()
        .context("model endpoint returned an error")?
        .json()
        .await
        .context("model endpoint response is not compatible with Chat Completions")?;
    let refined = response
        .choices
        .into_iter()
        .next()
        .context("model endpoint returned zero choices")?
        .message
        .content;
    if refined.trim().is_empty() {
        bail!("model endpoint returned an empty prompt");
    }
    Ok(refined)
}
