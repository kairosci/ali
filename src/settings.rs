use anyhow::{Context, Result};
use minijinja::Environment;
use regex::Regex;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct Config {
    pub adapters: Vec<crate::adapters::Adapter>,
    pub providers: Providers,
    pub product: Product,
    pub security: Security,
    pub embedding: Embedding,
    pub local_chat: LocalChat,
    pub storage: Storage,
    pub memory_policy: MemoryPolicy,
    pub web: WebPolicy,
    pub sensitivity: Vec<SensitivePattern>,
    pub documents: Documents,
    pub semantic_review: SemanticReview,
    pub agents: Vec<AgentDefinition>,
    root: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct Product {
    pub default_locale: String,
    pub supported_locales: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Security {
    pub default_profile: String,
    pub restricted_profiles: Vec<String>,
    pub outbound_default: String,
    pub max_handoff_lines: usize,
    pub max_document_bytes: u64,
    pub argon2_memory_kib: u32,
    pub argon2_iterations: u32,
    pub argon2_lanes: u32,
}

#[derive(Debug, Deserialize)]
pub struct Embedding {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub request_timeout_seconds: u64,
}

#[derive(Debug, Deserialize)]
pub struct LocalChat {
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Deserialize)]
pub struct Storage {
    pub qualifier: String,
    pub organization: String,
    pub application: String,
}

#[derive(Debug, Deserialize)]
pub struct MemoryPolicy {
    pub max_entry_bytes: u64,
    pub max_vault_bytes: u64,
    pub max_list_entries: usize,
    pub max_search_results: usize,
}

#[derive(Debug, Deserialize)]
pub struct WebPolicy {
    pub enabled: bool,
    pub session_seconds: u64,
    pub max_request_bytes: usize,
}

#[derive(Debug, Deserialize)]
pub struct Documents {
    pub allowed_text_extensions: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SemanticReview {
    pub chunk_chars: usize,
    pub max_chunks: usize,
    pub batch_size: usize,
    pub max_review_seconds: u64,
    pub similarity_threshold: f32,
    pub probes: Vec<SemanticProbe>,
}

#[derive(Debug, Deserialize)]
pub struct SemanticProbe {
    pub category: String,
    pub descriptions: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct AgentDefinition {
    pub id: String,
    pub executable: String,
    pub mode: String,
    pub prompt_profile: String,
    pub labels: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct SensitivePattern {
    pub category: String,
    pub regex: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Locale {
    pub messages: BTreeMap<String, String>,
}

impl Locale {
    pub fn message(&self, key: &str) -> Result<&str> {
        self.messages
            .get(key)
            .map(String::as_str)
            .ok_or_else(|| anyhow::anyhow!("locale is missing required message {key}"))
    }
}

#[derive(Debug, Deserialize)]
pub struct Providers {
    pub github_web_host: String,
    pub github_api_base: String,
    pub openai_compatible_path: String,
    pub request_timeout_seconds: u64,
    pub max_comments: u32,
}

impl Config {
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join("providers.toml");
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("cannot read configuration {}", path.display()))?;
        let provider_file: ProviderFile =
            toml::from_str(&raw).with_context(|| format!("cannot parse {}", path.display()))?;
        let runtime_path = root.join("runtime.toml");
        let runtime_raw = fs::read_to_string(&runtime_path)
            .with_context(|| format!("cannot read {}", runtime_path.display()))?;
        let runtime: RuntimeFile = toml::from_str(&runtime_raw)
            .with_context(|| format!("cannot parse {}", runtime_path.display()))?;
        let sensitivity_path = root.join("sensitivity.toml");
        let sensitivity_raw = fs::read_to_string(&sensitivity_path)
            .with_context(|| format!("cannot read {}", sensitivity_path.display()))?;
        let sensitivity_file: SensitivityFile = toml::from_str(&sensitivity_raw)
            .with_context(|| format!("cannot parse {}", sensitivity_path.display()))?;
        if sensitivity_file.patterns.is_empty() {
            anyhow::bail!("sensitivity.toml must define at least one detector");
        }
        for pattern in &sensitivity_file.patterns {
            Regex::new(&pattern.regex)
                .with_context(|| format!("invalid detector pattern {}", pattern.category))?;
        }
        let agents_path = root.join("agents.toml");
        let agents_raw = fs::read_to_string(&agents_path)
            .with_context(|| format!("cannot read {}", agents_path.display()))?;
        let agents_file: AgentsFile = toml::from_str(&agents_raw)
            .with_context(|| format!("cannot parse {}", agents_path.display()))?;
        if agents_file.agents.is_empty() {
            anyhow::bail!("agents.toml must define at least one agent");
        }
        for agent in &agents_file.agents {
            if agent.id.is_empty()
                || !agent
                    .id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            {
                anyhow::bail!("agent id contains unsupported characters");
            }
            if agent.executable.is_empty() || agent.executable.chars().any(char::is_whitespace) {
                anyhow::bail!("agent executable must be a single configured path or command name");
            }
            if agent.prompt_profile.is_empty()
                || !agent
                    .prompt_profile
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            {
                anyhow::bail!("agent prompt_profile must be a lowercase identifier");
            }
            if agent.mode != "inspect_only" {
                anyhow::bail!("only inspect_only agent adapters are currently supported");
            }
        }
        let review_path = root.join("privacy-review.toml");
        let review_raw = fs::read_to_string(&review_path)
            .with_context(|| format!("cannot read configuration {}", review_path.display()))?;
        let semantic_review: SemanticReview = toml::from_str(&review_raw)
            .with_context(|| format!("cannot parse {}", review_path.display()))?;
        let config = Self {
            adapters: crate::adapters::load(
                &root.join("adapters.toml"),
                &runtime.product.supported_locales,
            )?,
            providers: provider_file.providers,
            product: runtime.product,
            security: runtime.security,
            embedding: runtime.embedding,
            local_chat: runtime.local_chat,
            storage: runtime.storage,
            memory_policy: runtime.memory_policy,
            web: runtime.web,
            sensitivity: sensitivity_file.patterns,
            documents: runtime.documents,
            semantic_review,
            agents: agents_file.agents,
            root: root.to_owned(),
        };
        let api_base = url::Url::parse(&config.providers.github_api_base)
            .context("providers.github_api_base must be an absolute URL")?;
        if api_base.scheme() != "https" {
            anyhow::bail!("providers.github_api_base must use HTTPS");
        }
        if config.providers.github_web_host.trim().is_empty() {
            anyhow::bail!("providers.github_web_host cannot be empty");
        }
        if config.providers.request_timeout_seconds == 0 {
            anyhow::bail!("providers.request_timeout_seconds must be greater than zero");
        }
        if config.providers.max_comments == 0 {
            anyhow::bail!("providers.max_comments must be greater than zero");
        }
        if config.providers.max_comments > 100 {
            anyhow::bail!("providers.max_comments cannot exceed the GitHub API page limit of 100");
        }
        if !config.providers.openai_compatible_path.starts_with('/') {
            anyhow::bail!("providers.openai_compatible_path must start with a slash");
        }
        if config.security.outbound_default != "deny" {
            anyhow::bail!("security.outbound_default must be deny");
        }
        if config.security.max_handoff_lines == 0 || config.security.max_handoff_lines > 50 {
            anyhow::bail!("security.max_handoff_lines must be between 1 and 50");
        }
        if config.security.max_document_bytes == 0 {
            anyhow::bail!("security.max_document_bytes must be greater than zero");
        }
        if config.memory_policy.max_entry_bytes == 0
            || config.memory_policy.max_entry_bytes > config.security.max_document_bytes
            || config.memory_policy.max_vault_bytes < config.memory_policy.max_entry_bytes
            || config.memory_policy.max_vault_bytes
                > config.security.max_document_bytes.saturating_mul(100)
            || config.memory_policy.max_list_entries == 0
            || config.memory_policy.max_list_entries > 1000
            || config.memory_policy.max_search_results == 0
            || config.memory_policy.max_search_results > 100
        {
            anyhow::bail!("memory policy bounds are invalid");
        }
        if config.web.session_seconds == 0
            || config.web.session_seconds > 3600
            || config.web.max_request_bytes == 0
            || config.web.max_request_bytes > 65536
        {
            anyhow::bail!("local web session or request bounds are invalid");
        }
        if config.documents.allowed_text_extensions.is_empty()
            || config
                .documents
                .allowed_text_extensions
                .iter()
                .any(|extension| {
                    !extension.starts_with('.')
                        || extension.len() < 2
                        || extension
                            .bytes()
                            .any(|byte| !byte.is_ascii_alphanumeric() && byte != b'.')
                })
        {
            anyhow::bail!(
                "documents.allowed_text_extensions must be a non-empty list of simple extensions"
            );
        }
        if config.security.argon2_memory_kib < 65536
            || config.security.argon2_memory_kib > 1_048_576
            || config.security.argon2_iterations < 3
            || config.security.argon2_iterations > 10
            || config.security.argon2_lanes == 0
            || config.security.argon2_lanes > 16
        {
            anyhow::bail!("security Argon2 settings are below the minimum accepted policy");
        }
        if config.product.default_locale.is_empty()
            || config.product.supported_locales.is_empty()
            || !config
                .product
                .supported_locales
                .contains(&config.product.default_locale)
        {
            anyhow::bail!("product.default_locale must be in product.supported_locales");
        }
        if config.embedding.provider != "ollama" {
            anyhow::bail!("only the configured local Ollama embedding adapter is supported");
        }
        let embedding_url = url::Url::parse(&config.embedding.base_url)
            .context("embedding.base_url must be an absolute loopback URL")?;
        if embedding_url.scheme() != "http"
            || !embedding_url.username().is_empty()
            || embedding_url.password().is_some()
            || embedding_url.query().is_some()
            || embedding_url.fragment().is_some()
            || !embedding_url.host_str().is_some_and(|host| {
                host.parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
            })
        {
            anyhow::bail!("embedding endpoint must use a loopback host");
        }
        let chat_url = url::Url::parse(&config.local_chat.base_url)
            .context("local_chat.base_url must be an absolute loopback URL")?;
        if chat_url.scheme() != "http"
            || !chat_url.username().is_empty()
            || chat_url.password().is_some()
            || chat_url.query().is_some()
            || chat_url.fragment().is_some()
            || !chat_url.host_str().is_some_and(|host| {
                host.parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
            })
        {
            anyhow::bail!("local_chat endpoint must use a credential-free HTTP loopback URL");
        }
        if config.storage.qualifier.is_empty()
            || config.storage.organization.is_empty()
            || config.storage.application.is_empty()
        {
            anyhow::bail!("storage identifiers must be configured");
        }
        if config.embedding.request_timeout_seconds == 0 {
            anyhow::bail!("embedding.request_timeout_seconds must be greater than zero");
        }
        if config.semantic_review.chunk_chars < 100
            || config.semantic_review.chunk_chars > 8000
            || config.semantic_review.max_chunks == 0
            || config.semantic_review.max_chunks > 4096
            || config.semantic_review.batch_size == 0
            || config.semantic_review.batch_size > 32
            || config.semantic_review.max_review_seconds == 0
            || config.semantic_review.max_review_seconds > 600
            || !config.semantic_review.similarity_threshold.is_finite()
            || !(0.0..=1.0).contains(&config.semantic_review.similarity_threshold)
            || config.semantic_review.probes.is_empty()
            || config.semantic_review.probes.len() > 32
        {
            anyhow::bail!("privacy review chunk, batch, probe, or threshold settings are invalid");
        }
        for probe in &config.semantic_review.probes {
            if probe.category.is_empty()
                || !probe
                    .category
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
                || probe.descriptions.is_empty()
                || probe
                    .descriptions
                    .values()
                    .any(|text| text.trim().is_empty())
            {
                anyhow::bail!("privacy review probes need a category and non-empty descriptions");
            }
            for locale in &config.product.supported_locales {
                if !probe.descriptions.contains_key(locale) {
                    anyhow::bail!(
                        "privacy review probe {} lacks locale {locale}",
                        probe.category
                    );
                }
                let locale_data = config.locale(locale)?;
                if !locale_data
                    .messages
                    .contains_key(&format!("privacy_cue_{}", probe.category))
                {
                    anyhow::bail!(
                        "locale {locale} lacks privacy cue label for {}",
                        probe.category
                    );
                }
            }
        }
        for locale in &config.product.supported_locales {
            let mut templates = Environment::new();
            let prompt_path = format!("prompts/{locale}.md");
            let prompt = config.template(&prompt_path)?;
            templates
                .add_template(locale, &prompt)
                .with_context(|| format!("cannot parse {prompt_path} as a MiniJinja template"))?;
            let model_path = format!("model-system/{locale}.md");
            let model_template_name = format!("model-{locale}");
            let model_system = config.template(&model_path)?;
            templates
                .add_template(&model_template_name, &model_system)
                .with_context(|| format!("cannot parse {model_path} as a MiniJinja template"))?;
            let handoff_path = format!("handoffs/{locale}.md");
            let handoff = config.template(&handoff_path)?;
            templates
                .add_template(&format!("handoff-{locale}"), &handoff)
                .with_context(|| format!("cannot parse {handoff_path} as a MiniJinja template"))?;
            let help_path = format!("help/{locale}.md");
            if config.template(&help_path)?.trim().is_empty() {
                anyhow::bail!("configured help guide {help_path} is empty");
            }
            let locale_data = config.locale(locale)?;
            for key in REQUIRED_MESSAGES {
                if !locale_data.messages.contains_key(*key) {
                    anyhow::bail!("locale {locale} is missing required message {key}");
                }
            }
            for agent in &config.agents {
                if !agent.labels.contains_key(locale) {
                    anyhow::bail!("agent {} is missing a {locale} label", agent.id);
                }
                let agent_prompt_path =
                    format!("agent-prompts/{locale}/{}.md", agent.prompt_profile);
                let agent_prompt = config.template(&agent_prompt_path)?;
                if agent_prompt.trim().is_empty() {
                    anyhow::bail!("agent prompt template {agent_prompt_path} is empty");
                }
                let mut agent_templates = Environment::new();
                agent_templates
                    .add_template("agent", &agent_prompt)
                    .with_context(|| format!("cannot parse {agent_prompt_path} as a template"))?;
                agent_templates.get_template("agent")?;
            }
            for key in ["ui_agent_target", "ui_agent_generic"] {
                if !locale_data.messages.contains_key(key) {
                    anyhow::bail!("locale {locale} is missing required message {key}");
                }
            }
            for key in [
                "web_request_rejected",
                "web_invalid_request",
                "web_issue_failed",
                "web_prompt_failed",
            ] {
                if !locale_data.messages.contains_key(key) {
                    anyhow::bail!("locale {locale} is missing required message {key}");
                }
            }
            for asset in ["index.html", "app.js", "style.css"] {
                let path = format!("web/{locale}/{asset}");
                if config.template(&path)?.trim().is_empty() {
                    anyhow::bail!("web asset {path} is empty");
                }
            }
            for profile in &config.security.restricted_profiles {
                if !locale_data
                    .messages
                    .contains_key(&format!("profile_{profile}"))
                {
                    anyhow::bail!(
                        "locale {locale} is missing the label for restricted profile {profile}"
                    );
                }
            }
        }
        Ok(config)
    }

    pub fn template(&self, file: &str) -> Result<String> {
        let path = self.root.join(file);
        fs::read_to_string(&path)
            .with_context(|| format!("cannot read template {}", path.display()))
    }

    pub fn locale(&self, locale: &str) -> Result<Locale> {
        if !self
            .product
            .supported_locales
            .iter()
            .any(|value| value == locale)
        {
            anyhow::bail!("unsupported locale: {locale}");
        }
        let path = format!("locales/{locale}.toml");
        let raw = self.template(&path)?;
        toml::from_str(&raw).with_context(|| format!("cannot parse locale {locale}"))
    }
}

#[derive(Debug, Deserialize)]
struct RuntimeFile {
    product: Product,
    security: Security,
    embedding: Embedding,
    local_chat: LocalChat,
    storage: Storage,
    memory_policy: MemoryPolicy,
    web: WebPolicy,
    documents: Documents,
}

#[derive(Debug, Deserialize)]
struct ProviderFile {
    providers: Providers,
}

#[derive(Debug, Deserialize)]
struct SensitivityFile {
    patterns: Vec<SensitivePattern>,
}

#[derive(Debug, Deserialize)]
struct AgentsFile {
    agents: Vec<AgentDefinition>,
}

const REQUIRED_MESSAGES: &[&str] = &[
    "product_name",
    "prompt_written",
    "prompt_copied",
    "agent_destination_opened",
    "agent_env_notice",
    "outbound_blocked",
    "redaction_review",
    "memory_created",
    "memory_cleared",
    "handoff_created",
    "handoff_limit",
    "agent_not_configured",
    "language_missing",
    "local_embedding_unavailable",
    "document_too_large",
    "restricted_document",
    "handoff_review",
    "vault_passphrase",
    "vault_confirm_passphrase",
    "memory_input",
    "memory_query",
    "scan_found",
    "scan_clean",
    "redacted_file_written",
    "agents_list_header",
    "agent_found",
    "agent_not_found",
    "handoff_path",
    "ui_title",
    "ui_tagline",
    "ui_issue_url",
    "ui_prepare",
    "ui_copy",
    "ui_profile",
    "ui_general_profile",
    "ui_restricted_profile",
    "ui_document_input",
    "ui_document_output",
    "ui_scan",
    "ui_safety_notice",
    "ui_status_ready",
    "ui_status_loading",
    "ui_status_brief_ready",
    "ui_status_scan_done",
    "ui_model_endpoint",
    "ui_model_name",
    "ui_refine",
    "ui_refine_blocked",
    "ui_external_notice",
    "ui_open_agent_url",
    "ui_open_agent",
    "ui_busy",
    "profile_legal",
    "profile_development",
    "profile_judicial",
    "profile_health",
    "profile_finance",
    "profile_identity",
    "ui_language_label",
    "ui_missing_issue_url",
    "ui_job_disconnected",
    "privacy_analysis_complete",
    "privacy_analysis_no_cues",
    "privacy_analysis_warning",
    "privacy_cue_detail",
    "privacy_cue_personal_identifiers",
    "privacy_cue_credentials_access",
    "privacy_cue_legal_case_details",
    "privacy_cue_health_information",
    "privacy_cue_financial_account_data",
    "privacy_cue_confidential_work",
    "ui_semantic_analyze",
    "ui_semantic_warning",
    "memory_shared_label",
    "ui_memory_heading",
    "ui_memory_passphrase",
    "ui_memory_confirmation",
    "ui_memory_initialize",
    "ui_memory_mismatch",
    "ui_memory_agent",
    "ui_memory_text",
    "ui_memory_add",
    "ui_memory_list",
    "ui_memory_query",
    "ui_memory_search",
    "ui_memory_searched",
    "ui_memory_listed",
    "ui_memory_working",
    "ui_memory_initialized",
    "ui_memory_added",
    "ui_memory_local_notice",
];
