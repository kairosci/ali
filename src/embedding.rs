use crate::settings::Config;
use anyhow::{Context, Result, bail};
use reqwest::redirect::Policy;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize)]
struct EmbedRequest<'a> {
    model: &'a str,
    input: &'a [&'a str],
}

#[derive(Deserialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

pub async fn embed(text: &str, config: &Config) -> Result<Vec<f32>> {
    let mut vectors = embed_many(&[text], config).await?;
    Ok(vectors.remove(0))
}

pub async fn embed_many(texts: &[&str], config: &Config) -> Result<Vec<Vec<f32>>> {
    if texts.is_empty() {
        bail!("embedding batch must not be empty");
    }
    let base = config.embedding.base_url.trim_end_matches('/');
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(
            config.embedding.request_timeout_seconds,
        ))
        .no_proxy()
        .redirect(Policy::none())
        .build()?;
    let response: EmbedResponse = client
        .post(format!("{base}/api/embed"))
        .json(&EmbedRequest {
            model: &config.embedding.model,
            input: texts,
        })
        .send()
        .await
        .context("local embedding request failed; no remote fallback is permitted")?
        .error_for_status()
        .context("local embedding service rejected the request")?
        .json()
        .await
        .context("local embedding response is invalid")?;
    let vectors = response.embeddings;
    if vectors.len() != texts.len() {
        bail!("embedding service returned the wrong number of vectors");
    }
    if vectors
        .iter()
        .any(|vector| vector.is_empty() || vector.iter().any(|value| !value.is_finite()))
    {
        bail!("embedding service returned an invalid vector");
    }
    let dimension = vectors[0].len();
    if vectors.iter().any(|vector| vector.len() != dimension) {
        bail!("embedding service returned inconsistent vector dimensions");
    }
    Ok(vectors)
}
