mod adapters;
mod agents;
mod embedding;
mod gui;
mod handoff;
mod issue;
mod memory;
mod privacy;
mod prompt;
mod provider;
mod settings;
mod web;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use std::{fs, path::PathBuf};

#[derive(Debug, Parser)]
#[command(
    name = "ali",
    version,
    about = "Prepare grounded coding-agent prompts from issue URLs"
)]
struct Cli {
    /// Directory containing parsed TOML settings and Markdown templates.
    #[arg(long, global = true, env = "ALI_CONFIG_DIR", default_value = "config")]
    config_dir: PathBuf,
    /// Select one configured interface and prompt language.
    #[arg(long, global = true)]
    language: Option<String>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Fetch an issue and create a ready-to-paste implementation brief.
    Prepare {
        /// GitHub issue or pull request URL.
        url: String,
        /// Write the prompt to this file instead of stdout.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Copy the prompt to the system clipboard.
        #[arg(long)]
        copy: bool,
        /// Open this agent URL after preparing the prompt (paste remains manual).
        #[arg(long, env = "ALI_AGENT_URL")]
        open: Option<String>,
        /// Ask an OpenAI-compatible endpoint (including local Ollama) to analyze the issue.
        #[arg(long)]
        model_endpoint: Option<String>,
        /// Model identifier sent to the compatible endpoint.
        #[arg(long)]
        model: Option<String>,
        /// Data handling profile from config/runtime.toml.
        #[arg(long)]
        profile: Option<String>,
        /// Select a configured agent-specific prompt profile.
        #[arg(long)]
        agent_id: Option<String>,
    },
    /// Manage the encrypted, local semantic memory vault.
    Memory {
        /// Override the OS-specific private data directory.
        #[arg(long)]
        data_dir: Option<PathBuf>,
        #[command(subcommand)]
        action: MemoryCommand,
    },
    /// Scan a UTF-8 text file locally and write a separate redacted candidate.
    Privacy {
        #[command(subcommand)]
        action: PrivacyCommand,
    },
    /// Create a bounded, locally scanned handoff from schema-validated JSON on stdin.
    Handoff {
        #[command(subcommand)]
        action: HandoffCommand,
    },
    /// List only the agent CLIs explicitly declared in config/agents.toml.
    Agents {
        #[command(subcommand)]
        action: AgentCommand,
    },
    /// Launch the native local ALI interface.
    Gui,
    /// Show the configuration-driven model protocol catalog.
    Adapters {
        #[command(subcommand)]
        action: AdapterCommand,
    },
    /// Start the loopback-only browser interface with a one-use local session capability.
    Web,
}

#[derive(Debug, Subcommand)]
enum MemoryCommand {
    Init,
    Add {
        /// Associate this note with a configured agent identifier.
        #[arg(long)]
        agent_id: Option<String>,
    },
    Search {
        /// Search shared notes and notes associated with this configured agent.
        #[arg(long)]
        agent_id: Option<String>,
    },
    List,
    Clear,
}

#[derive(Debug, Subcommand)]
enum PrivacyCommand {
    Scan {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Analyze {
        #[arg(long)]
        input: PathBuf,
    },
    Review {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum HandoffCommand {
    Create {
        /// Write outside the private local session directory.
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum AgentCommand {
    List,
}

#[derive(Debug, Subcommand)]
enum AdapterCommand {
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    if print_configured_help()? {
        return Ok(());
    }
    let cli = Cli::parse();
    match cli.command {
        Commands::Prepare {
            url,
            output,
            copy,
            open,
            model_endpoint,
            model,
            profile,
            agent_id,
        } => {
            let config = settings::Config::load(&cli.config_dir)?;
            let language = cli
                .language
                .as_deref()
                .unwrap_or(&config.product.default_locale);
            let locale = config.locale(language)?;
            let profile = profile
                .as_deref()
                .unwrap_or(&config.security.default_profile);
            let known_profile = profile == "general"
                || config
                    .security
                    .restricted_profiles
                    .iter()
                    .any(|value| value == profile);
            if !known_profile {
                bail!("unknown data profile: {profile}");
            }
            if agent_id
                .as_ref()
                .is_some_and(|id| !config.agents.iter().any(|agent| agent.id == *id))
            {
                bail!("agent id must match an entry in config/agents.toml");
            }
            if profile != "general" && (copy || open.is_some()) {
                bail!("{}", locale.message("outbound_blocked")?);
            }
            let issue = issue::fetch(&url, &config).await?;
            let prompt = prompt::build_for_agent(&issue, &config, language, agent_id.as_deref())?;
            let prompt = if let Some(endpoint) = model_endpoint {
                if profile != "general" {
                    bail!("{}", locale.message("restricted_document")?);
                }
                let model = model.context("--model is required with --model-endpoint")?;
                provider::refine(&endpoint, &model, &prompt, &config, language).await?
            } else {
                prompt
            };

            if let Some(path) = output {
                fs::write(&path, &prompt)
                    .with_context(|| format!("cannot write {}", path.display()))?;
                eprintln!(
                    "{}",
                    locale
                        .message("prompt_written")?
                        .replace("{path}", &path.display().to_string())
                );
            } else {
                print!("{prompt}");
            }
            if copy {
                copy_to_clipboard(&prompt, &locale)?;
            }
            if let Some(target) = open {
                open_target(&target, &locale)?;
            }
        }
        Commands::Memory { data_dir, action } => {
            let config = settings::Config::load(&cli.config_dir)?;
            let language = cli
                .language
                .as_deref()
                .unwrap_or(&config.product.default_locale);
            let locale = config.locale(language)?;
            let action = match action {
                MemoryCommand::Init => memory::Action::Init,
                MemoryCommand::Add { agent_id } => memory::Action::Add { agent_id },
                MemoryCommand::Search { agent_id } => memory::Action::Search { agent_id },
                MemoryCommand::List => memory::Action::List,
                MemoryCommand::Clear => memory::Action::Clear,
            };
            memory::run(action, &config, &locale, data_dir.as_deref()).await?;
        }
        Commands::Privacy { action } => {
            let config = settings::Config::load(&cli.config_dir)?;
            let language = cli
                .language
                .as_deref()
                .unwrap_or(&config.product.default_locale);
            let locale = config.locale(language)?;
            match action {
                PrivacyCommand::Scan { input, output } => {
                    let report = privacy::scan_file(&input, &output, &config)?;
                    if report.findings.is_empty() {
                        eprintln!("{}", locale.message("scan_clean")?);
                    } else {
                        let categories = report
                            .findings
                            .keys()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", ");
                        let count: usize = report.findings.values().sum();
                        eprintln!(
                            "{}",
                            locale
                                .message("scan_found")?
                                .replace("{count}", &count.to_string())
                                .replace("{categories}", &categories)
                        );
                    }
                    println!(
                        "{}",
                        locale
                            .message("redacted_file_written")?
                            .replace("{path}", &report.output_path.display().to_string())
                    );
                    eprintln!("{}", locale.message("redaction_review")?);
                }
                PrivacyCommand::Analyze { input } => {
                    let report = privacy::analyze_file(&input, &config).await?;
                    println!("{}", privacy::format_semantic_report(&report, &locale)?);
                    eprintln!("{}", locale.message("privacy_analysis_warning")?);
                }
                PrivacyCommand::Review { input, output } => {
                    let report = privacy::review_file(&input, &output, &config).await?;
                    let categories = report
                        .findings
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ");
                    let count: usize = report.findings.values().sum();
                    if count == 0 {
                        eprintln!("{}", locale.message("scan_clean")?);
                    } else {
                        eprintln!(
                            "{}",
                            locale
                                .message("scan_found")?
                                .replace("{count}", &count.to_string())
                                .replace("{categories}", &categories)
                        );
                    }
                    println!(
                        "{}",
                        locale
                            .message("redacted_file_written")?
                            .replace("{path}", &report.output_path.display().to_string())
                    );
                    println!(
                        "{}",
                        privacy::format_semantic_report(&report.semantic, &locale)?
                    );
                    eprintln!("{}", locale.message("privacy_analysis_warning")?);
                    eprintln!("{}", locale.message("redaction_review")?);
                }
            }
        }
        Commands::Handoff { action } => {
            let config = settings::Config::load(&cli.config_dir)?;
            let language = cli
                .language
                .as_deref()
                .unwrap_or(&config.product.default_locale);
            let locale = config.locale(language)?;
            match action {
                HandoffCommand::Create { output } => {
                    handoff::create(&config, &locale, language, output.as_deref())?;
                }
            }
        }
        Commands::Agents { action } => {
            let config = settings::Config::load(&cli.config_dir)?;
            let language = cli
                .language
                .as_deref()
                .unwrap_or(&config.product.default_locale);
            let locale = config.locale(language)?;
            match action {
                AgentCommand::List => agents::list(&config, &locale, language)?,
            }
        }
        Commands::Gui => {
            gui::run(cli.config_dir, cli.language.as_deref())?;
        }
        Commands::Adapters { action } => {
            let config = settings::Config::load(&cli.config_dir)?;
            let language = cli
                .language
                .as_deref()
                .unwrap_or(&config.product.default_locale);
            config.locale(language)?;
            match action {
                AdapterCommand::List => adapters::list(&config.adapters, language)?,
            }
        }
        Commands::Web => {
            web::run(cli.config_dir, cli.language.as_deref()).await?;
        }
    }
    Ok(())
}

fn print_configured_help() -> Result<bool> {
    use std::{env, ffi::OsStr};

    let args: Vec<_> = env::args_os().skip(1).collect();
    let requested = args.is_empty()
        || args.iter().any(|arg| {
            arg == OsStr::new("--help") || arg == OsStr::new("-h") || arg == OsStr::new("help")
        });
    if !requested {
        return Ok(false);
    }

    let mut config_dir = env::var_os("ALI_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("config"));
    let mut language = None;
    let mut index = 0;
    while index < args.len() {
        let value = args[index].to_string_lossy();
        if let Some(path) = value.strip_prefix("--config-dir=") {
            config_dir = PathBuf::from(path);
        } else if value == "--config-dir" {
            if let Some(path) = args.get(index + 1) {
                config_dir = PathBuf::from(path);
                index += 1;
            }
        } else if let Some(code) = value.strip_prefix("--language=") {
            language = Some(code.to_owned());
        } else if value == "--language" {
            if let Some(code) = args.get(index + 1) {
                language = Some(code.to_string_lossy().into_owned());
                index += 1;
            }
        }
        index += 1;
    }
    let config = settings::Config::load(&config_dir)?;
    let language = language.unwrap_or_else(|| config.product.default_locale.clone());
    config.locale(&language)?;
    print!("{}", config.template(&format!("help/{language}.md"))?);
    Ok(true)
}

fn copy_to_clipboard(text: &str, locale: &settings::Locale) -> Result<()> {
    let mut clipboard =
        arboard::Clipboard::new().context("could not connect to the system clipboard")?;
    clipboard
        .set_text(text.to_owned())
        .context("could not write the prompt to the system clipboard")?;
    eprintln!("{}", locale.message("prompt_copied")?);
    Ok(())
}

pub(crate) fn open_target(target: &str, locale: &settings::Locale) -> Result<()> {
    let parsed = url::Url::parse(target).context("--open must be an absolute URL")?;
    if !matches!(parsed.scheme(), "https" | "http") {
        bail!("--open only supports HTTP and HTTPS URLs");
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        bail!("--open URL must not contain embedded credentials");
    }
    webbrowser::open(parsed.as_str()).context("could not launch the browser")?;
    eprintln!("{}", locale.message("agent_destination_opened")?);
    Ok(())
}
