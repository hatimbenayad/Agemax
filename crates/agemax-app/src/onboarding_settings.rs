use crate::chrome_ui::{centered_modal_pos, modal_frame};
use agemax_agy::adapter::{AgentAdapter, AgentLaunchPreset, AntigravityAdapter};
use agemax_agy::profile::{ProfileConfig, ProfileManager};
use agemax_config::keybindings::KeybindingsConfig;
use agemax_config::schema::{AgeMaxConfig, CustomCommand, UiMode};
use agemax_config::theme::{ThemeConfig, ThemePreset};
use agemax_core::layout::PaneId;
use agemax_term::pty::{available_shells, resolve_shell_program};
use crossbeam_channel::Receiver;
use eframe::egui::{self, Color32, Key, Stroke, Vec2};
use std::path::PathBuf;

pub const OFFICIAL_AGY_INSTALL_CMD: &str = "irm https://antigravity.google/cli/install.ps1 | iex";

/// Background preflight check results (`REQ-075`).
#[derive(Debug, Clone)]
pub struct PreflightResult {
    pub agy_binary: Option<PathBuf>,
    pub resolved_shell: PathBuf,
}

/// Manages asynchronous startup dependency discovery without blocking the UI loop.
pub struct PreflightState {
    pub result: Option<PreflightResult>,
    pub session_dismissed: bool,
    pub copied_feedback: bool,
    rx: Option<Receiver<PreflightResult>>,
}

impl Default for PreflightState {
    fn default() -> Self {
        Self::new()
    }
}

impl PreflightState {
    pub fn new() -> Self {
        Self {
            result: None,
            session_dismissed: false,
            copied_feedback: false,
            rx: None,
        }
    }

    pub fn trigger_check(&mut self, default_shell: Option<String>, ctx: Option<egui::Context>) {
        let (tx, rx) = crossbeam_channel::bounded(1);
        std::thread::Builder::new()
            .name("agemax-preflight".to_string())
            .spawn(move || {
                let agy_binary = AntigravityAdapter::new().detect_binary();
                let resolved_shell = resolve_shell_program(default_shell.as_deref());
                let _ = tx.send(PreflightResult {
                    agy_binary,
                    resolved_shell,
                });
                if let Some(c) = ctx {
                    c.request_repaint();
                }
            })
            .ok();
        self.rx = Some(rx);
    }

    pub fn poll(&mut self) {
        if let Some(rx) = &self.rx {
            if let Ok(res) = rx.try_recv() {
                self.result = Some(res);
                self.rx = None;
            }
        }
    }
}

pub enum OnboardingAction {
    Recheck,
    HideForever,
    Dismiss,
}

/// Render non-blocking setup guide banner if `agy.exe` was not detected (`REQ-075`).
pub fn render_onboarding_banner(
    ui: &mut egui::Ui,
    state: &mut PreflightState,
    config: &AgeMaxConfig,
) -> Option<OnboardingAction> {
    if !config.general.enable_onboarding_banner || state.session_dismissed {
        return None;
    }
    let res = state.result.as_ref()?;
    if res.agy_binary.is_some() {
        return None;
    }

    let mut action = None;
    let frame = egui::Frame::new()
        .fill(Color32::from_rgb(42, 34, 22))
        .stroke(Stroke::new(1.0, Color32::from_rgb(200, 145, 50)))
        .inner_margin(Vec2::new(10.0, 6.0));

    frame.show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new("Antigravity CLI (agy) wasn't found yet.")
                    .size(12.0)
                    .color(Color32::from_rgb(255, 220, 150))
                    .strong(),
            );
            ui.label(
                egui::RichText::new("Normal terminals work now. To install agy in PowerShell:")
                    .size(11.5)
                    .color(Color32::from_rgb(220, 210, 195)),
            );

            let copy_label = if state.copied_feedback {
                "Copied ✓"
            } else {
                "Copy Install Command"
            };
            if ui.button(copy_label).clicked() {
                ui.ctx().copy_text(OFFICIAL_AGY_INSTALL_CMD.to_string());
                state.copied_feedback = true;
            }
            if ui.button("Re-check").clicked() {
                state.copied_feedback = false;
                action = Some(OnboardingAction::Recheck);
            }
            if ui.small_button("Hide Forever").clicked() {
                action = Some(OnboardingAction::HideForever);
            }
            if ui.small_button("Dismiss").clicked() {
                action = Some(OnboardingAction::Dismiss);
            }
        });
    });

    action
}

/// Request returned when a user selects a preset card inside `AgentLauncherModal`.
pub struct AgentLaunchRequest {
    pub target_pane: PaneId,
    pub preset: AgentLaunchPreset,
    pub profile: ProfileConfig,
}

/// Plain-English visual launcher modal for starting `agy` without memorizing flags (`REQ-076`).
#[derive(Debug)]
pub struct AgentLauncherModal {
    pub is_open: bool,
    pub target_pane: Option<PaneId>,
    profiles: Vec<ProfileConfig>,
    selected_profile_idx: usize,
    creating_profile: bool,
    new_profile_name: String,
    new_profile_shared_history: bool,
    profile_error: Option<String>,
}

impl Default for AgentLauncherModal {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentLauncherModal {
    pub fn new() -> Self {
        Self {
            is_open: false,
            target_pane: None,
            profiles: vec![ProfileConfig::default_mode_a()],
            selected_profile_idx: 0,
            creating_profile: false,
            new_profile_name: String::new(),
            new_profile_shared_history: true,
            profile_error: None,
        }
    }

    pub fn open(&mut self, target_pane: PaneId) {
        self.is_open = true;
        self.target_pane = Some(target_pane);
        self.profiles = ProfileManager::new().list_profiles();
        if self.selected_profile_idx >= self.profiles.len() {
            self.selected_profile_idx = 0;
        }
        self.creating_profile = false;
        self.new_profile_name.clear();
        self.profile_error = None;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.creating_profile = false;
        self.profile_error = None;
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        preflight: &mut PreflightState,
    ) -> Option<AgentLaunchRequest> {
        if !self.is_open {
            return None;
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.close();
            return None;
        }

        let target_pane = self.target_pane?;
        let mut launch_req = None;
        let mut should_close = false;
        let pos = centered_modal_pos(ctx, 500.0, 390.0);

        egui::Window::new("agent_launcher_modal")
            .fixed_pos(pos)
            .fixed_size(Vec2::new(500.0, 380.0))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .frame(modal_frame())
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Start Antigravity Agent")
                                .size(15.5)
                                .color(Color32::WHITE)
                                .strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Close").clicked() {
                                should_close = true;
                            }
                        });
                    });

                    ui.label(
                        egui::RichText::new("Choose how you want to start the AI agent—no flags to memorize.")
                            .size(11.5)
                            .color(Color32::from_rgb(160, 170, 190)),
                    );

                    let missing_agy = preflight
                        .result
                        .as_ref()
                        .map(|r| r.agy_binary.is_none())
                        .unwrap_or(false);
                    if missing_agy {
                        ui.add_space(6.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                egui::RichText::new("⚠ agy.exe not detected yet.")
                                    .size(11.5)
                                    .color(Color32::from_rgb(245, 175, 45))
                                    .strong(),
                            );
                            if ui.small_button("Copy Install Command").clicked() {
                                ctx.copy_text(OFFICIAL_AGY_INSTALL_CMD.to_string());
                                preflight.copied_feedback = true;
                            }
                            if ui.small_button("Re-check").clicked() {
                                preflight.trigger_check(None, Some(ctx.clone()));
                            }
                        });
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(6.0);

                    // Profile picker row (Mode A default vs Mode B isolated directory)
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Account Profile:")
                                .size(12.0)
                                .color(Color32::from_rgb(210, 215, 230))
                                .strong(),
                        );
                        for (idx, prof) in self.profiles.iter().enumerate() {
                            let label = if prof.isolate_storage {
                                format!("{} (Isolated)", prof.name)
                            } else {
                                "Default Login".to_string()
                            };
                            if ui
                                .selectable_label(self.selected_profile_idx == idx, label)
                                .clicked()
                            {
                                self.selected_profile_idx = idx;
                            }
                        }
                        if ui.small_button("+ Profile").clicked() {
                            self.creating_profile = !self.creating_profile;
                            self.profile_error = None;
                        }
                    });

                    if self.creating_profile {
                        ui.add_space(4.0);
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label("Name:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.new_profile_name)
                                        .desired_width(140.0),
                                );
                                ui.checkbox(
                                    &mut self.new_profile_shared_history,
                                    "Share history & plugins",
                                );
                                if ui.button("Create").clicked() {
                                    let trimmed = self.new_profile_name.trim();
                                    if trimmed.is_empty()
                                        || !trimmed
                                            .chars()
                                            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
                                    {
                                        self.profile_error =
                                            Some("Use letters, numbers, '-' or '_'.".to_string());
                                    } else {
                                        match ProfileManager::new().create_profile(
                                            trimmed,
                                            None,
                                            self.new_profile_shared_history,
                                        ) {
                                            Ok(created) => {
                                                self.profiles = ProfileManager::new().list_profiles();
                                                self.selected_profile_idx = self
                                                    .profiles
                                                    .iter()
                                                    .position(|p| p.name == created.name)
                                                    .unwrap_or(0);
                                                self.new_profile_name.clear();
                                                self.creating_profile = false;
                                                self.profile_error = None;
                                            }
                                            Err(e) => {
                                                self.profile_error = Some(e.to_string());
                                            }
                                        }
                                    }
                                }
                            });
                            ui.label(
                                egui::RichText::new("First sign-in opens directly inside the official agy terminal (Age Max never reads passwords).")
                                    .size(10.5)
                                    .color(Color32::from_rgb(145, 155, 175)),
                            );
                            if let Some(err) = &self.profile_error {
                                ui.label(
                                    egui::RichText::new(err)
                                        .size(11.0)
                                        .color(Color32::from_rgb(235, 95, 95)),
                                );
                            }
                        });
                    }

                    ui.add_space(10.0);
                    let active_profile = self
                        .profiles
                        .get(self.selected_profile_idx)
                        .cloned()
                        .unwrap_or_else(ProfileConfig::default_mode_a);

                    let cards = [
                        (
                            "▶ Start New Agent Session",
                            "Launches a fresh Antigravity conversation (agy) ready to read and work in your folder.",
                            AgentLaunchPreset::NewSession,
                        ),
                        (
                            "🛡 Safe Plan Mode (Recommended for Beginners)",
                            "Asks the AI to show you a step-by-step plan first before making edits (agy --mode plan).",
                            AgentLaunchPreset::SafePlanMode,
                        ),
                        (
                            "↻ Continue Last Conversation",
                            "Resumes your most recent Antigravity chat in this project directory (agy -c).",
                            AgentLaunchPreset::ContinueLast,
                        ),
                    ];

                    for (title, subtitle, preset) in cards {
                        let card_frame = egui::Frame::new()
                            .fill(Color32::from_rgb(31, 36, 52))
                            .corner_radius(6.0)
                            .stroke(Stroke::new(1.0, Color32::from_rgb(55, 65, 90)))
                            .inner_margin(10.0);
                        card_frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(title)
                                            .size(13.0)
                                            .color(Color32::WHITE)
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(subtitle)
                                            .size(11.0)
                                            .color(Color32::from_rgb(165, 175, 195)),
                                    );
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.button("Launch").clicked() {
                                            launch_req = Some(AgentLaunchRequest {
                                                target_pane,
                                                preset: preset.clone(),
                                                profile: active_profile.clone(),
                                            });
                                            should_close = true;
                                        }
                                    },
                                );
                            });
                        });
                        ui.add_space(6.0);
                    }
                });
            });

        if should_close {
            self.close();
        }
        launch_req
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    Appearance,
    TerminalAlerts,
    Shortcuts,
    QuickActions,
}

/// Comprehensive 4-tab visual Settings Modal backed cleanly by `config.toml` (`REQ-077`).
#[derive(Debug, Default)]
pub struct SettingsModal {
    pub is_open: bool,
    active_tab: SettingsTab,
    draft_keybindings: KeybindingsConfig,
    keybinding_status: Option<Result<(), String>>,
    new_cmd_name: String,
    new_cmd_line: String,
    new_cmd_desc: String,
}

impl SettingsModal {
    pub fn open(&mut self, config: &AgeMaxConfig) {
        self.is_open = true;
        self.draft_keybindings = config.keybindings.clone();
        self.keybinding_status = None;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.keybinding_status = None;
    }

    /// Show Settings UI. Returns `true` when `config` changed and should be saved/applied.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        config: &mut AgeMaxConfig,
        preflight: &mut PreflightState,
    ) -> bool {
        if !self.is_open {
            return false;
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.close();
            return false;
        }

        let mut changed = false;
        let mut should_close = false;
        let pos = centered_modal_pos(ctx, 560.0, 500.0);

        egui::Window::new("settings_modal")
            .fixed_pos(pos)
            .fixed_size(Vec2::new(560.0, 490.0))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .frame(modal_frame())
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Age Max Settings")
                                .size(15.5)
                                .color(Color32::WHITE)
                                .strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Close").clicked() {
                                should_close = true;
                            }
                        });
                    });

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        for (tab, label) in [
                            (SettingsTab::Appearance, "Appearance & View"),
                            (SettingsTab::TerminalAlerts, "Terminal & Alerts"),
                            (SettingsTab::Shortcuts, "Shortcuts"),
                            (SettingsTab::QuickActions, "Quick Actions"),
                        ] {
                            if ui.selectable_label(self.active_tab == tab, label).clicked() {
                                self.active_tab = tab;
                            }
                        }
                    });
                    ui.separator();
                    ui.add_space(6.0);

                    egui::ScrollArea::vertical()
                        .max_height(385.0)
                        .show(ui, |ui| match self.active_tab {
                            SettingsTab::Appearance => {
                                if render_appearance_tab(ui, config) {
                                    changed = true;
                                }
                            }
                            SettingsTab::TerminalAlerts => {
                                if render_terminal_alerts_tab(ui, config, preflight) {
                                    changed = true;
                                }
                            }
                            SettingsTab::Shortcuts => {
                                if render_shortcuts_tab(
                                    ui,
                                    &mut self.draft_keybindings,
                                    &mut self.keybinding_status,
                                    &mut config.keybindings,
                                ) {
                                    changed = true;
                                }
                            }
                            SettingsTab::QuickActions => {
                                if render_quick_actions_tab(
                                    ui,
                                    &mut config.commands,
                                    &mut self.new_cmd_name,
                                    &mut self.new_cmd_line,
                                    &mut self.new_cmd_desc,
                                ) {
                                    changed = true;
                                }
                            }
                        });
                });
            });

        if should_close {
            self.close();
        }
        changed
    }
}

fn render_appearance_tab(ui: &mut egui::Ui, config: &mut AgeMaxConfig) -> bool {
    let mut changed = false;
    ui.label(
        egui::RichText::new("UI Chrome Mode")
            .size(13.0)
            .color(Color32::WHITE)
            .strong(),
    );
    ui.label(
        egui::RichText::new(
            "Switch freely between button-first Guided mode or distraction-free Minimal mode.",
        )
        .size(11.0)
        .color(Color32::from_rgb(160, 170, 190)),
    );
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui
            .selectable_label(
                config.general.ui_mode == UiMode::Guided,
                "Guided (Toolbar, Pane Headers & Tips)",
            )
            .clicked()
            && config.general.ui_mode != UiMode::Guided
        {
            config.general.ui_mode = UiMode::Guided;
            config.general.show_toolbar = None;
            config.general.show_pane_headers = None;
            config.general.show_shortcut_hints = None;
            changed = true;
        }
        if ui
            .selectable_label(
                config.general.ui_mode == UiMode::Minimal,
                "Minimal (Clean Terminal Canvas)",
            )
            .clicked()
            && config.general.ui_mode != UiMode::Minimal
        {
            config.general.ui_mode = UiMode::Minimal;
            config.general.show_toolbar = None;
            config.general.show_pane_headers = None;
            config.general.show_shortcut_hints = None;
            changed = true;
        }
    });

    ui.add_space(6.0);
    let mut toolbar = config.general.effective_show_toolbar();
    if ui
        .checkbox(&mut toolbar, "Show Top Action Toolbar")
        .changed()
    {
        config.general.show_toolbar = Some(toolbar);
        changed = true;
    }
    let mut headers = config.general.effective_show_pane_headers();
    if ui
        .checkbox(&mut headers, "Show Per-Pane Status Headers & Quick Buttons")
        .changed()
    {
        config.general.show_pane_headers = Some(headers);
        changed = true;
    }
    let mut hints = config.general.effective_show_shortcut_hints();
    if ui
        .checkbox(&mut hints, "Show Shortcut Tooltips & Hints")
        .changed()
    {
        config.general.show_shortcut_hints = Some(hints);
        changed = true;
    }

    ui.add_space(10.0);
    ui.label(
        egui::RichText::new("Theme Preset")
            .size(13.0)
            .color(Color32::WHITE)
            .strong(),
    );
    ui.horizontal(|ui| {
        for (preset, label) in [
            (ThemePreset::MidnightDark, "Midnight Dark"),
            (ThemePreset::HighContrast, "High Contrast"),
            (ThemePreset::SoftLight, "Soft Light"),
        ] {
            let preset_cfg = ThemeConfig::from_preset(preset);
            if ui
                .selectable_label(config.theme == preset_cfg, label)
                .clicked()
            {
                config.theme = preset_cfg;
                changed = true;
            }
        }
    });

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.label("Terminal Font Size:");
        if ui
            .add(egui::Slider::new(&mut config.general.font_size, 10.0..=26.0).step_by(1.0))
            .changed()
        {
            changed = true;
        }
    });
    changed
}

fn render_terminal_alerts_tab(
    ui: &mut egui::Ui,
    config: &mut AgeMaxConfig,
    preflight: &mut PreflightState,
) -> bool {
    let mut changed = false;

    ui.label(
        egui::RichText::new("Default Terminal Shell (for newly opened panes)")
            .size(13.0)
            .color(Color32::WHITE)
            .strong(),
    );
    ui.add_space(4.0);
    let current_shell = config
        .general
        .default_shell
        .as_deref()
        .unwrap_or("auto")
        .to_string();
    for (id, label) in available_shells() {
        let selected = current_shell.eq_ignore_ascii_case(id);
        if ui.selectable_label(selected, label).clicked() {
            config.general.default_shell = if id == "auto" {
                None
            } else {
                Some(id.to_string())
            };
            preflight.trigger_check(config.general.default_shell.clone(), Some(ui.ctx().clone()));
            changed = true;
        }
    }

    ui.add_space(10.0);
    ui.label(
        egui::RichText::new("Attention & Onboarding")
            .size(13.0)
            .color(Color32::WHITE)
            .strong(),
    );
    if ui
        .checkbox(
            &mut config.general.enable_toast_notifications,
            "Show Windows Toast popups when an agent needs input or finishes",
        )
        .changed()
    {
        changed = true;
    }
    if ui
        .checkbox(
            &mut config.general.enable_taskbar_flash,
            "Flash taskbar icon on agent attention events",
        )
        .changed()
    {
        changed = true;
    }
    if ui
        .checkbox(
            &mut config.general.enable_onboarding_banner,
            "Show setup guide banner if agy.exe is not installed",
        )
        .changed()
    {
        changed = true;
    }

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new("Detected Environment")
            .size(12.5)
            .color(Color32::WHITE)
            .strong(),
    );
    let agy_text = preflight
        .result
        .as_ref()
        .and_then(|r| r.agy_binary.as_ref())
        .map(|p| format!("agy.exe: {}", p.to_string_lossy()))
        .unwrap_or_else(|| "agy.exe: Not detected".to_string());
    let shell_text = preflight
        .result
        .as_ref()
        .map(|r| format!("Resolved shell: {}", r.resolved_shell.to_string_lossy()))
        .unwrap_or_else(|| "Resolved shell: Checking...".to_string());
    ui.label(
        egui::RichText::new(agy_text)
            .size(11.0)
            .color(Color32::GRAY),
    );
    ui.label(
        egui::RichText::new(shell_text)
            .size(11.0)
            .color(Color32::GRAY),
    );
    ui.label(
        egui::RichText::new(format!(
            "Config file: {}",
            AgeMaxConfig::default_path().to_string_lossy()
        ))
        .size(11.0)
        .color(Color32::GRAY),
    );
    if ui.small_button("Re-check agy.exe").clicked() {
        preflight.trigger_check(config.general.default_shell.clone(), Some(ui.ctx().clone()));
    }

    changed
}

fn render_shortcuts_tab(
    ui: &mut egui::Ui,
    draft: &mut KeybindingsConfig,
    status: &mut Option<Result<(), String>>,
    live: &mut KeybindingsConfig,
) -> bool {
    let mut changed = false;
    ui.label(
        egui::RichText::new("Customize Shortcuts (examples: Ctrl+Shift+T, Ctrl+B, Alt+Left)")
            .size(11.5)
            .color(Color32::from_rgb(165, 175, 195)),
    );
    ui.add_space(6.0);

    egui::Grid::new("settings_shortcuts_grid")
        .num_columns(2)
        .spacing([20.0, 5.0])
        .show(ui, |ui| {
            let fields = [
                ("New Tab", &mut draft.new_tab),
                ("Split Right", &mut draft.split_horizontal),
                ("Split Down", &mut draft.split_vertical),
                ("Close Pane", &mut draft.close_pane),
                ("Zoom / Unzoom", &mut draft.zoom_pane),
                ("Toggle Sidebar", &mut draft.toggle_sidebar),
                ("Command Palette", &mut draft.command_palette),
                ("Jump to Waiting Agent", &mut draft.jump_unread),
                ("Toggle Alerts Panel", &mut draft.toggle_notifications),
            ];
            for (label, value) in fields {
                ui.label(label);
                ui.add(egui::TextEdit::singleline(value).desired_width(170.0));
                ui.end_row();
            }
        });

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button("Apply Shortcuts").clicked() {
            match draft.validate_all() {
                Ok(()) => {
                    *live = draft.clone();
                    *status = Some(Ok(()));
                    changed = true;
                }
                Err(err) => {
                    *status = Some(Err(err));
                }
            }
        }
        if ui.button("Reset Defaults").clicked() {
            *draft = KeybindingsConfig::default();
            *live = draft.clone();
            *status = Some(Ok(()));
            changed = true;
        }
    });

    if let Some(res) = status {
        ui.add_space(4.0);
        match res {
            Ok(()) => {
                ui.label(
                    egui::RichText::new("✓ Shortcuts saved and active.")
                        .size(11.5)
                        .color(Color32::from_rgb(75, 215, 135)),
                );
            }
            Err(err) => {
                ui.label(
                    egui::RichText::new(format!("✕ {}", err))
                        .size(11.5)
                        .color(Color32::from_rgb(245, 95, 95)),
                );
            }
        }
    }
    changed
}

fn render_quick_actions_tab(
    ui: &mut egui::Ui,
    commands: &mut Vec<CustomCommand>,
    new_name: &mut String,
    new_line: &mut String,
    new_desc: &mut String,
) -> bool {
    let mut changed = false;
    ui.label(
        egui::RichText::new(
            "Custom Quick Actions appear directly inside ⌘ Actions (Command Palette).",
        )
        .size(11.5)
        .color(Color32::from_rgb(165, 175, 195)),
    );
    ui.add_space(6.0);

    let mut remove_idx = None;
    for (idx, cmd) in commands.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(&cmd.name).strong());
            ui.label(
                egui::RichText::new(format!("-> {}", cmd.command))
                    .monospace()
                    .color(Color32::from_rgb(140, 180, 240)),
            );
            if ui.small_button("Remove").clicked() {
                remove_idx = Some(idx);
            }
        });
    }
    if let Some(idx) = remove_idx {
        commands.remove(idx);
        changed = true;
    }

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new("Add Quick Action")
            .size(12.5)
            .color(Color32::WHITE)
            .strong(),
    );
    egui::Grid::new("add_quick_action_grid")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            ui.label("Button Name:");
            ui.add(
                egui::TextEdit::singleline(new_name)
                    .hint_text("e.g. Run Project Tests")
                    .desired_width(240.0),
            );
            ui.end_row();

            ui.label("Shell Command:");
            ui.add(
                egui::TextEdit::singleline(new_line)
                    .hint_text("e.g. npm test")
                    .desired_width(240.0),
            );
            ui.end_row();

            ui.label("Description:");
            ui.add(
                egui::TextEdit::singleline(new_desc)
                    .hint_text("Optional note")
                    .desired_width(240.0),
            );
            ui.end_row();
        });

    let valid = !new_name.trim().is_empty() && !new_line.trim().is_empty();
    if ui
        .add_enabled(valid, egui::Button::new("+ Add Quick Action"))
        .clicked()
    {
        commands.push(CustomCommand {
            name: new_name.trim().to_string(),
            description: if new_desc.trim().is_empty() {
                None
            } else {
                Some(new_desc.trim().to_string())
            },
            command: new_line.trim().to_string(),
            working_dir: None,
            keybinding: None,
        });
        new_name.clear();
        new_line.clear();
        new_desc.clear();
        changed = true;
    }

    changed
}
