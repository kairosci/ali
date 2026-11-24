use crate::{
    memory, privacy,
    settings::{Config, Locale},
};
use anyhow::{Context, Result, bail};
use minijinja::{Environment, context};
use serde::Deserialize;
use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    path::Path,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HandoffInput {
    session_id: String,
    failed_agent: String,
    failure: String,
    verified_progress: Vec<String>,
    next_action: String,
}

pub fn create(
    config: &Config,
    locale: &Locale,
    language: &str,
    output: Option<&Path>,
) -> Result<()> {
    let mut bytes = Vec::new();
    io::stdin()
        .take(config.security.max_document_bytes.min(65536) + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > config.security.max_document_bytes.min(65536) {
        bail!("handoff input exceeds its configured size limit");
    }
    let input: HandoffInput = serde_json::from_slice(&bytes)
        .context("handoff input must be valid JSON matching the ALI schema")?;
    validate_identifier(&input.session_id, "session_id", 64)?;
    validate_identifier(&input.failed_agent, "failed_agent", 64)?;
    if !config
        .agents
        .iter()
        .any(|agent| agent.id == input.failed_agent)
    {
        bail!("failed_agent must match an agent configured in config/agents.toml");
    }
    if ![
        "crash",
        "timeout",
        "non_zero_exit",
        "token_limit",
        "cancelled",
        "unknown",
    ]
    .contains(&input.failure.as_str())
    {
        bail!("failure must be one of the configured handoff failure codes");
    }
    if input.verified_progress.len() > 8 {
        bail!("handoff can contain at most eight verified progress items");
    }
    for value in input.verified_progress.iter().chain([&input.next_action]) {
        if value.trim().is_empty()
            || value.chars().count() > 240
            || value.chars().any(char::is_control)
        {
            bail!("handoff text must be non-empty, single-line, and at most 240 characters");
        }
    }
    let source = config.template(&format!("handoffs/{language}.md"))?;
    let mut templates = Environment::new();
    templates.add_template("handoff", &source)?;
    let template = templates.get_template("handoff")?;
    let rendered = template.render(context! {
        session_id => input.session_id,
        agent => input.failed_agent,
        failure => input.failure,
        verified_progress => input.verified_progress,
        next_action => input.next_action,
    })?;
    let scan = privacy::redact(&rendered, config)?;
    let line_count = scan.redacted.lines().count();
    if line_count > config.security.max_handoff_lines {
        bail!("{}", locale.message("handoff_limit")?);
    }
    let root = memory::default_data_dir(config)?.join("sessions");
    ensure_private_dir(&root)?;
    let destination = match output {
        Some(path) => absolute_path(path)?,
        None => root.join(format!("handoff-{}.md", input.session_id)),
    };
    let parent = destination
        .parent()
        .context("handoff output path has no parent")?;
    if !parent.exists() {
        bail!("handoff output directory does not exist");
    }
    let metadata =
        fs::symlink_metadata(parent).context("cannot inspect handoff output directory")?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        bail!("handoff output parent must be a real directory");
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&destination)
        .context("handoff already exists or cannot be created")?;
    file.write_all(scan.redacted.as_bytes())?;
    file.sync_all()?;
    if !scan.findings.is_empty() {
        eprintln!("{}", locale.message("redaction_review")?);
    }
    println!(
        "{}",
        locale
            .message("handoff_path")?
            .replace("{path}", &destination.display().to_string())
            .replace("{lines}", &line_count.to_string())
    );
    Ok(())
}

fn validate_identifier(value: &str, field: &str, max: usize) -> Result<()> {
    if value.is_empty()
        || value.len() > max
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        bail!("{field} must contain only ASCII letters, digits, hyphens, or underscores");
    }
    Ok(())
}

fn absolute_path(path: &Path) -> Result<std::path::PathBuf> {
    if path.is_absolute() {
        Ok(path.to_owned())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn ensure_private_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            let mut builder = fs::DirBuilder::new();
            builder.recursive(true).mode(0o700).create(path)?;
        }
        #[cfg(not(unix))]
        fs::create_dir_all(path)?;
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        bail!("handoff directory must not be a symlink");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            bail!("handoff directory permissions are too broad");
        }
    }
    Ok(())
}
