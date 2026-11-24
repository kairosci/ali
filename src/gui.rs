use crate::{
    issue, privacy, prompt, provider,
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

type JobResult = std::result::Result<JobOutput, String>;

enum JobOutput {
    Brief(String),
    SemanticReview { summary: String, redacted: String },
}

pub fn run(config_dir: PathBuf, language_override: Option<&str>) -> Result<()> {
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
                config_dir, language, locale, config,
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
    profile: String,
    prompt: String,
    model_endpoint: String,
    model: String,
    agent_url: String,
    document_input: String,
    document_output: String,
    redacted_preview: String,
    semantic_result: String,
    status: String,
    job: Option<Receiver<JobResult>>,
}

impl Workbench {
    fn new(config_dir: PathBuf, language: String, locale: Locale, config: Config) -> Result<Self> {
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
            prompt: String::new(),
            agent_url: String::new(),
            document_input: String::new(),
            document_output: String::new(),
            redacted_preview: String::new(),
            semantic_result: String::new(),
            status,
            job: None,
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
                    prompt::build(&issue, &config, &language)
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
