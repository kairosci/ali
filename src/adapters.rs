use anyhow::{Context, Result};
use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Debug, Deserialize)]
pub struct Adapter {
    pub id: String,
    pub protocol: String,
    pub status: String,
    pub locality: String,
    pub labels: BTreeMap<String, String>,
}

#[derive(Deserialize)]
struct Catalog {
    adapters: Vec<Adapter>,
}

pub fn load(path: &Path, locales: &[String]) -> Result<Vec<Adapter>> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("cannot read adapter catalog {}", path.display()))?;
    let catalog: Catalog = toml::from_str(&raw)
        .with_context(|| format!("cannot parse adapter catalog {}", path.display()))?;
    if catalog.adapters.is_empty() {
        anyhow::bail!("adapter catalog must contain at least one entry");
    }
    let mut ids = std::collections::BTreeSet::new();
    for adapter in &catalog.adapters {
        if adapter.id.is_empty()
            || !adapter
                .id
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            || !ids.insert(&adapter.id)
        {
            anyhow::bail!("adapter ids must be unique lowercase identifiers");
        }
        if adapter.protocol.is_empty()
            || !adapter
                .protocol
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            anyhow::bail!("adapter protocol must be a lowercase identifier");
        }
        if !matches!(
            adapter.status.as_str(),
            "implemented" | "experimental" | "planned"
        ) {
            anyhow::bail!("adapter status is not recognized");
        }
        if !matches!(adapter.locality.as_str(), "local" | "remote" | "either") {
            anyhow::bail!("adapter locality is not recognized");
        }
        for locale in locales {
            if adapter
                .labels
                .get(locale)
                .is_none_or(|label| label.trim().is_empty())
            {
                anyhow::bail!("adapter {} is missing a {locale} label", adapter.id);
            }
        }
    }
    Ok(catalog.adapters)
}

pub fn list(adapters: &[Adapter], locale: &str) -> Result<()> {
    for adapter in adapters {
        let label = adapter
            .labels
            .get(locale)
            .context("validated adapter label is missing")?;
        println!(
            "{}\t{}\t{}\t{}",
            adapter.id, adapter.status, adapter.locality, label
        );
    }
    Ok(())
}
