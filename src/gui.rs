use crate::{
    issue, memory, privacy, prompt, provider,
    settings::{Config, Locale},
};
use anyhow::Result;
use eframe::egui;
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};
use zeroize::Zeroizing;

type JobResult = std::result::Result<JobOutput, String>;

enum JobOutput {
    Brief(String),
    SemanticReview { summary: String, redacted: String },
    Memory { status: String, details: String },
}

enum MemoryAction {
    Initialize {
        passphrase: Zeroizing<String>,
    },
    Add {
        passphrase: Zeroizing<String>,
        text: Zeroizing<String>,
        agent_id: Option<String>,
    },
    List {
        passphrase: Zeroizing<String>,
    },
    Search {
        passphrase: Zeroizing<String>,
        query: Zeroizing<String>,
        agent_id: Option<String>,
    },
}

pub fn run(
    config_dir: PathBuf,
    language_override: Option<&str>,
    smoke_test: bool,
    smoke_test_input: Option<PathBuf>,
    smoke_test_output: Option<PathBuf>,
) -> Result<()> {
    if smoke_test_input.is_some() != smoke_test_output.is_some() {
        anyhow::bail!("GUI smoke scan requires both input and output paths");
    }
    if !smoke_test && smoke_test_input.is_some() {
        anyhow::bail!("GUI smoke scan paths require smoke-test mode");
    }
    let config = Config::load(&config_dir)?;
    let language = language_override
        .unwrap_or(&config.product.default_locale)
        .to_owned();
    let locale = config.locale(&language)?;
    let title = locale.message("ui_title")?.to_owned();
    eframe::run_native(
        &title,
        eframe::NativeOptions::default(),
        Box::new(move |_creation| {
            Ok(Box::new(Workbench::new(
                config_dir,
                language,
                locale,
                config,
                smoke_test,
                smoke_test_input,
                smoke_test_output,
            )?))
        }),
    )
    .map_err(|error| anyhow::anyhow!("native GUI failed: {error}"))
}

struct Workbench {
    config_dir: PathBuf,
    config: Config,
    locale: Locale,
    language: String,
    issue_url: String,
    agent_id: String,
    profile: String,
    prompt: String,
    model_endpoint: String,
    model: String,
    agent_url: String,
    document_input: String,
    document_output: String,
    redacted_preview: String,
    semantic_result: String,
    memory_passphrase: String,
    memory_confirmation: String,
    memory_text: String,
    memory_query: String,
    memory_agent_id: String,
    memory_results: String,
    status: String,
    job: Option<Receiver<JobResult>>,
    close_after_first_frame: bool,
    smoke_scan: Option<(PathBuf, PathBuf)>,
}

impl Workbench {
    fn new(
        config_dir: PathBuf,
        language: String,
        locale: Locale,
        config: Config,
        smoke_test: bool,
        smoke_test_input: Option<PathBuf>,
        smoke_test_output: Option<PathBuf>,
    ) -> Result<Self> {
        let status = locale.message("ui_status_ready")?.to_owned();
        Ok(Self {
            config_dir,
            profile: config.security.default_profile.clone(),
            model_endpoint: config.local_chat.base_url.clone(),
            model: config.local_chat.model.clone(),
            config,
            locale,
            language,
            issue_url: String::new(),
            agent_id: String::new(),
            prompt: String::new(),
            agent_url: String::new(),
            document_input: smoke_test_input
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            document_output: smoke_test_output
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            redacted_preview: String::new(),
            semantic_result: String::new(),
            memory_passphrase: String::new(),
            memory_confirmation: String::new(),
            memory_text: String::new(),
            memory_query: String::new(),
            memory_agent_id: String::new(),
            memory_results: String::new(),
            status,
            job: None,
            close_after_first_frame: smoke_test,
            smoke_scan: smoke_test_input.zip(smoke_test_output),
        })
    }

    fn message(&self, key: &str) -> String {
        self.locale
            .message(key)
            .map(str::to_owned)
            .unwrap_or_else(|error| error.to_string())
    }

    fn start_prepare(&mut self) {
        if self.job.is_some() {
            self.status = self.message("ui_busy").to_owned();
            return;
        }
        if self.issue_url.trim().is_empty() {
            self.status = self.message("ui_missing_issue_url");
            return;
        }
        let url = self.issue_url.clone();
        let agent_id = (!self.agent_id.is_empty()).then(|| self.agent_id.clone());
        let config_dir = self.config_dir.clone();
        let language = self.language.clone();
        let (sender, receiver) = mpsc::channel();
        self.job = Some(receiver);
        self.status = self.message("ui_status_loading").to_owned();
        thread::spawn(move || {
            let result = (|| -> Result<String> {
                let config = Config::load(&config_dir)?;
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                runtime.block_on(async {
                    let issue = issue::fetch(&url, &config).await?;
                    prompt::build_for_agent(&issue, &config, &language, agent_id.as_deref())
                })
            })()
            .map_err(|error| error.to_string());
            let _ = sender.send(result.map(JobOutput::Brief));
        });
    }

    fn start_refine(&mut self) {
        if self.profile != "general" {
            self.status = self.message("ui_refine_blocked").to_owned();
            return;
        }
        if self.job.is_some() {
            self.status = self.message("ui_busy").to_owned();
            return;
        }
        if self.prompt.is_empty() {
            self.status = self.message("ui_status_ready").to_owned();
            return;
        }
        let prompt = self.prompt.clone();
        let endpoint = self.model_endpoint.clone();
        let model = self.model.clone();
        let language = self.language.clone();
        let config_dir = self.config_dir.clone();
        let (sender, receiver) = mpsc::channel();
        self.job = Some(receiver);
        self.status = self.message("ui_status_loading").to_owned();
        thread::spawn(move || {
            let result = (|| -> Result<String> {
                let config = Config::load(&config_dir)?;
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                runtime.block_on(provider::refine(
                    &endpoint, &model, &prompt, &config, &language,
                ))
            })()
            .map_err(|error| error.to_string());
            let _ = sender.send(result.map(JobOutput::Brief));
        });
    }

    fn poll_job(&mut self) {
        let Some(receiver) = &self.job else {
            return;
        };
        match receiver.try_recv() {
            Ok(Ok(JobOutput::Brief(prompt))) => {
                self.prompt = prompt;
                self.status = self.message("ui_status_brief_ready").to_owned();
                self.job = None;
            }
            Ok(Ok(JobOutput::SemanticReview { summary, redacted })) => {
                self.semantic_result = summary;
                self.redacted_preview = redacted;
                self.status = self.message("ui_status_scan_done");
                self.job = None;
            }
            Ok(Ok(JobOutput::Memory { status, details })) => {
                self.status = status;
                self.memory_results = details;
                self.job = None;
            }
            Ok(Err(error)) => {
                self.status = error;
                self.job = None;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.status = self.message("ui_job_disconnected");
                self.job = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }

    fn scan_document(&mut self) {
        if self.document_input.trim().is_empty() || self.document_output.trim().is_empty() {
            self.status = self.message("ui_document_input").to_owned();
            return;
        }
        match privacy::scan_file(
            &PathBuf::from(&self.document_input),
            &PathBuf::from(&self.document_output),
            &self.config,
        ) {
            Ok(report) => {
                self.redacted_preview = report.redacted;
                self.status = self.message("ui_status_scan_done").to_owned();
            }
            Err(error) => self.status = error.to_string(),
        }
    }

    fn start_semantic_review(&mut self) {
        if self.job.is_some() {
            self.status = self.message("ui_busy");
            return;
        }
        if self.document_input.trim().is_empty() || self.document_output.trim().is_empty() {
            self.status = self.message("ui_document_input");
            return;
        }
        let path = PathBuf::from(&self.document_input);
        let output = PathBuf::from(&self.document_output);
        let config_dir = self.config_dir.clone();
        let language = self.language.clone();
        let (sender, receiver) = mpsc::channel();
        self.job = Some(receiver);
        self.status = self.message("ui_status_loading");
        thread::spawn(move || {
            let result = (|| -> Result<(String, String)> {
                let config = Config::load(&config_dir)?;
                let locale = config.locale(&language)?;
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                let report = runtime.block_on(privacy::review_file(&path, &output, &config))?;
                let summary = privacy::format_semantic_report(&report.semantic, &locale)?;
                Ok((summary, report.redacted))
            })()
            .map(|(summary, redacted)| JobOutput::SemanticReview { summary, redacted })
            .map_err(|error: anyhow::Error| error.to_string());
            let _ = sender.send(result);
        });
    }

    fn start_memory(&mut self, action: MemoryAction) {
        use zeroize::Zeroize;
        self.memory_confirmation.zeroize();
        if self.job.is_some() {
            self.status = self.message("ui_busy");
            return;
        }
        let config_dir = self.config_dir.clone();
        let language = self.language.clone();
        let (sender, receiver) = mpsc::channel();
        self.job = Some(receiver);
        self.status = self.message("ui_memory_working");
        thread::spawn(move || {
            let result = (|| -> Result<(String, String)> {
                let config = Config::load(&config_dir)?;
                let locale = config.locale(&language)?;
                let root = memory::default_data_dir(&config)?;
                match action {
                    MemoryAction::Initialize { passphrase } => {
                        memory::initialize_with_passphrase(&root, passphrase, &config)?;
                        Ok((
                            locale.message("ui_memory_initialized")?.to_owned(),
                            String::new(),
                        ))
                    }
                    MemoryAction::Add {
                        passphrase,
                        text,
                        agent_id,
                    } => {
                        let runtime = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()?;
                        runtime.block_on(memory::add_with_passphrase(
                            &root,
                            text,
                            passphrase,
                            agent_id.as_deref(),
                            &config,
                        ))?;
                        Ok((locale.message("ui_memory_added")?.to_owned(), String::new()))
                    }
                    MemoryAction::List { passphrase } => {
                        let entries = memory::list_with_passphrase(&root, passphrase, &config)?;
                        let shared = locale.message("memory_shared_label")?;
                        let details = entries
                            .iter()
                            .map(|(id, agent, text)| {
                                format!(
                                    "{id}\t{}\t{}",
                                    if agent.is_empty() { shared } else { agent },
                                    text.as_str()
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        Ok((locale.message("ui_memory_listed")?.to_owned(), details))
                    }
                    MemoryAction::Search {
                        passphrase,
                        query,
                        agent_id,
                    } => {
                        let runtime = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()?;
                        let entries = runtime.block_on(memory::search_with_passphrase(
                            &root,
                            query,
                            passphrase,
                            agent_id.as_deref(),
                            &config,
                        ))?;
                        let shared = locale.message("memory_shared_label")?;
                        let details = entries
                            .iter()
                            .map(|(score, id, agent, text)| {
                                format!(
                                    "{score:.4}\t{id}\t{}\t{}",
                                    if agent.is_empty() { shared } else { agent },
                                    text.as_str()
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        Ok((locale.message("ui_memory_searched")?.to_owned(), details))
                    }
                }
            })()
            .map(|(status, details)| JobOutput::Memory { status, details })
            .map_err(|error: anyhow::Error| error.to_string());
            let _ = sender.send(result);
        });
    }
}

impl Drop for Workbench {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.memory_passphrase.zeroize();
        self.memory_confirmation.zeroize();
        self.memory_text.zeroize();
        self.memory_query.zeroize();
        self.memory_results.zeroize();
        self.prompt.zeroize();
        self.redacted_preview.zeroize();
        self.semantic_result.zeroize();
    }
}

impl eframe::App for Workbench {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_job();
        egui::TopBottomPanel::top("header").show(context, |ui| {
            ui.heading(self.message("ui_title"));
            ui.label(self.message("ui_tagline"));
            ui.label(self.message("ui_external_notice"));
        });
        egui::CentralPanel::default().show(context, |ui| {
            ui.horizontal(|ui| {
                ui.label(self.message("ui_language_label"));
                egui::ComboBox::from_id_salt("language")
                    .selected_text(&self.language)
                    .show_ui(ui, |ui| {
                        for code in &self.config.product.supported_locales {
                            if ui
                                .selectable_value(&mut self.language, code.clone(), code)
                                .clicked()
                            {
                                match self.config.locale(code) {
                                    Ok(locale) => self.locale = locale,
                                    Err(error) => self.status = error.to_string(),
                                }
                            }
                        }
                    });
            });
            ui.separator();
            ui.label(self.message("ui_profile"));
            egui::ComboBox::from_id_salt("profile")
                .selected_text(self.profile_label())
                .show_ui(ui, |ui| {
                    let general_label = self.message("ui_general_profile");
                    ui.selectable_value(&mut self.profile, "general".to_owned(), general_label);
                    for profile in self.config.security.restricted_profiles.clone() {
                        let label = self.profile_key(&profile).to_owned();
                        let text = self.message(&label).to_owned();
                        ui.selectable_value(&mut self.profile, profile, text);
                    }
                });
            ui.separator();
            ui.label(self.message("ui_agent_target"));
            egui::ComboBox::from_id_salt("prompt-agent")
                .selected_text(if self.agent_id.is_empty() {
                    self.message("ui_agent_generic")
                } else {
                    self.config
                        .agents
                        .iter()
                        .find(|agent| agent.id == self.agent_id)
                        .and_then(|agent| agent.labels.get(&self.language))
                        .cloned()
                        .unwrap_or_default()
                })
                .show_ui(ui, |ui| {
                    let generic_label = self.message("ui_agent_generic");
                    ui.selectable_value(&mut self.agent_id, String::new(), generic_label);
                    for agent in &self.config.agents {
                        if let Some(label) = agent.labels.get(&self.language) {
                            ui.selectable_value(&mut self.agent_id, agent.id.clone(), label);
                        }
                    }
                });
            ui.separator();
            ui.label(self.message("ui_issue_url"));
            ui.text_edit_singleline(&mut self.issue_url);
            ui.horizontal(|ui| {
                if ui.button(self.message("ui_prepare")).clicked() {
                    self.start_prepare();
                }
                if ui
                    .add_enabled(
                        self.profile == "general" && !self.prompt.is_empty(),
                        egui::Button::new(self.message("ui_copy")),
                    )
                    .clicked()
                {
                    context.copy_text(self.prompt.clone());
                }
            });
            ui.label(self.message("ui_model_endpoint"));
            ui.text_edit_singleline(&mut self.model_endpoint);
            ui.label(self.message("ui_model_name"));
            ui.text_edit_singleline(&mut self.model);
            if ui
                .add_enabled(
                    self.profile == "general",
                    egui::Button::new(self.message("ui_refine")),
                )
                .clicked()
            {
                self.start_refine();
            }
            if self.profile != "general" {
                ui.label(self.message("ui_refine_blocked"));
            }
            egui::ScrollArea::vertical()
                .max_height(250.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.prompt)
                            .desired_rows(10)
                            .desired_width(f32::INFINITY),
                    );
                });
            ui.separator();
            ui.label(self.message("ui_document_input"));
            ui.text_edit_singleline(&mut self.document_input);
            ui.label(self.message("ui_document_output"));
            ui.text_edit_singleline(&mut self.document_output);
            if ui.button(self.message("ui_scan")).clicked() {
                self.scan_document();
            }
            if ui.button(self.message("ui_semantic_analyze")).clicked() {
                self.start_semantic_review();
            }
            ui.label(self.message("ui_semantic_warning"));
            egui::ScrollArea::vertical()
                .max_height(140.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.semantic_result)
                            .desired_rows(4)
                            .desired_width(f32::INFINITY),
                    );
                });
            ui.label(self.message("ui_safety_notice"));
            egui::ScrollArea::vertical()
                .max_height(180.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.redacted_preview)
                            .desired_rows(6)
                            .desired_width(f32::INFINITY),
                    );
                });
            ui.separator();
            ui.collapsing(self.message("ui_memory_heading"), |ui| {
                ui.label(self.message("ui_memory_passphrase"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.memory_passphrase)
                        .password(true)
                        .desired_width(f32::INFINITY),
                );
                ui.label(self.message("ui_memory_confirmation"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.memory_confirmation)
                        .password(true)
                        .desired_width(f32::INFINITY),
                );
                ui.horizontal(|ui| {
                    if ui.button(self.message("ui_memory_initialize")).clicked() {
                        let first = Zeroizing::new(std::mem::take(&mut self.memory_passphrase));
                        let second = Zeroizing::new(std::mem::take(&mut self.memory_confirmation));
                        if first != second {
                            self.status = self.message("ui_memory_mismatch");
                        } else {
                            self.start_memory(MemoryAction::Initialize { passphrase: first });
                        }
                    }
                });
                ui.label(self.message("ui_memory_agent"));
                egui::ComboBox::from_id_salt("memory-agent")
                    .selected_text(if self.memory_agent_id.is_empty() {
                        self.message("memory_shared_label")
                    } else {
                        self.config
                            .agents
                            .iter()
                            .find(|agent| agent.id == self.memory_agent_id)
                            .and_then(|agent| agent.labels.get(&self.language))
                            .cloned()
                            .unwrap_or_default()
                    })
                    .show_ui(ui, |ui| {
                        let shared_label = self.message("memory_shared_label");
                        ui.selectable_value(&mut self.memory_agent_id, String::new(), shared_label);
                        for agent in &self.config.agents {
                            if let Some(label) = agent.labels.get(&self.language) {
                                ui.selectable_value(
                                    &mut self.memory_agent_id,
                                    agent.id.clone(),
                                    label,
                                );
                            }
                        }
                    });
                ui.label(self.message("ui_memory_text"));
                ui.add(
                    egui::TextEdit::multiline(&mut self.memory_text)
                        .desired_rows(3)
                        .desired_width(f32::INFINITY),
                );
                ui.horizontal(|ui| {
                    if ui.button(self.message("ui_memory_add")).clicked() {
                        let passphrase =
                            Zeroizing::new(std::mem::take(&mut self.memory_passphrase));
                        let text = Zeroizing::new(std::mem::take(&mut self.memory_text));
                        let agent_id = (!self.memory_agent_id.is_empty())
                            .then(|| self.memory_agent_id.clone());
                        self.start_memory(MemoryAction::Add {
                            passphrase,
                            text,
                            agent_id,
                        });
                    }
                    if ui.button(self.message("ui_memory_list")).clicked() {
                        let passphrase =
                            Zeroizing::new(std::mem::take(&mut self.memory_passphrase));
                        self.start_memory(MemoryAction::List { passphrase });
                    }
                });
                ui.label(self.message("ui_memory_query"));
                ui.text_edit_singleline(&mut self.memory_query);
                if ui.button(self.message("ui_memory_search")).clicked() {
                    let passphrase = Zeroizing::new(std::mem::take(&mut self.memory_passphrase));
                    let query = Zeroizing::new(std::mem::take(&mut self.memory_query));
                    let agent_id =
                        (!self.memory_agent_id.is_empty()).then(|| self.memory_agent_id.clone());
                    self.start_memory(MemoryAction::Search {
                        passphrase,
                        query,
                        agent_id,
                    });
                }
                ui.label(self.message("ui_memory_local_notice"));
                egui::ScrollArea::vertical()
                    .max_height(120.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.memory_results)
                                .desired_rows(4)
                                .desired_width(f32::INFINITY),
                        );
                    });
            });
            ui.separator();
            ui.label(self.message("ui_open_agent_url"));
            ui.text_edit_singleline(&mut self.agent_url);
            if ui
                .add_enabled(
                    self.profile == "general" && !self.agent_url.is_empty(),
                    egui::Button::new(self.message("ui_open_agent")),
                )
                .clicked()
            {
                match crate::open_target(&self.agent_url, &self.locale) {
                    Ok(()) => self.status = self.message("agent_destination_opened").to_owned(),
                    Err(error) => self.status = error.to_string(),
                }
            }
            ui.separator();
            ui.label(&self.status);
        });
        if self.job.is_some() {
            context.request_repaint_after(Duration::from_millis(50));
        }
        if self.close_after_first_frame {
            if self.smoke_scan.take().is_some() {
                self.scan_document();
            }
            self.close_after_first_frame = false;
            context.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

impl Workbench {
    fn profile_key(&self, profile: &str) -> String {
        format!("profile_{profile}")
    }

    fn profile_label(&self) -> String {
        if self.profile == "general" {
            return self.message("ui_general_profile").to_owned();
        }
        self.message(&self.profile_key(&self.profile)).to_owned()
    }
}
