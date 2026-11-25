use crate::folder_picker::spawn_folder_picker_async;
use crate::icons::{icon_label, icon_rich, icon_small, icon_text, status_icon, UiIcon};
use crate::ui_tokens::{metrics, UiTokens};
use crate::visual_alive::{render_agent_mascot, ThemeChromeColors};
pub use agemax_config::keybindings::KeybindingsConfig;
use agemax_config::safety::{display_clean_path, validate_workspace_path};
use agemax_config::schema::{AgeMaxConfig, UiMode};
use agemax_config::theme::ResolvedTheme;
use agemax_core::layout::PaneId;
use agemax_core::status::AgentStatus;
use agemax_core::workspace::WorkspaceBackend;
use agemax_term::detect_wsl_distros;
use crossbeam_channel::Receiver;
use eframe::egui::{self, Color32, Key, Pos2, Rect, Stroke, Vec2};
use std::path::{Path, PathBuf};

/// Actions emitted by the Guided mode Top Utility Toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarAction {
    ToggleSidebar,
    OpenNewWorkspace,
    NewTab,
    SplitRight,
    SplitDown,
    LaunchAgy,
    OpenBrowserPane,
    OpenAcpPanel,
    LaunchTeamGrid,
    ToggleGridView,
    OpenBroadcast,
    OpenWelcome,
    OpenCommandPalette,
    ToggleNotifications,
    OpenViewSettings,
    OpenSafetySettings,
    SwitchUiMode(UiMode),
}

/// Actions emitted by a Guided mode per-pane mini header bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneHeaderAction {
    LaunchAgy(PaneId),
    SplitRight(PaneId),
    SplitDown(PaneId),
    ToggleZoom(PaneId),
    Close(PaneId),
}

/// Target item being renamed in `RenameModal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameTarget {
    Workspace(usize),
    Tab { ws_index: usize, tab_index: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackendTab {
    #[default]
    Local,
    Wsl,
    Ssh,
}

#[derive(Debug, Clone)]
pub struct NewWorkspaceRequest {
    pub name: String,
    pub cwd: PathBuf,
    pub launch_agy: bool,
    pub backend: WorkspaceBackend,
}

/// Modal state for creating a Workspace across Local, WSL, or Remote SSH backends.
#[derive(Debug)]
pub struct NewWorkspaceModal {
    pub is_open: bool,
    pub name: String,
    pub path_input: String,
    pub name_manually_edited: bool,
    pub launch_agy: bool,
    backend_tab: BackendTab,
    wsl_distro: String,
    wsl_linux_cwd: String,
    detected_wsl_distros: Vec<String>,
    ssh_target: String,
    ssh_port: String,
    ssh_remote_dir: String,
    picker_rx: Option<Receiver<Option<PathBuf>>>,
}

impl Default for NewWorkspaceModal {
    fn default() -> Self {
        Self::new()
    }
}

impl NewWorkspaceModal {
    pub fn new() -> Self {
        Self {
            is_open: false,
            name: String::new(),
            path_input: String::new(),
            name_manually_edited: false,
            launch_agy: false,
            backend_tab: BackendTab::Local,
            wsl_distro: String::new(),
            wsl_linux_cwd: "~".to_string(),
            detected_wsl_distros: Vec::new(),
            ssh_target: String::new(),
            ssh_port: String::new(),
            ssh_remote_dir: "~".to_string(),
            picker_rx: None,
        }
    }

    pub fn open(&mut self, default_dir: &Path, default_index: usize, allowed_root: Option<&Path>) {
        self.is_open = true;
        self.backend_tab = BackendTab::Local;
        let initial_dir = allowed_root
            .filter(|root| validate_workspace_path(default_dir, Some(root)).is_err())
            .unwrap_or(default_dir);
        self.path_input = display_clean_path(initial_dir);
        self.name = initial_dir
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|s| !s.trim().is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("Workspace {}", default_index));
        self.name_manually_edited = false;
        self.launch_agy = false;
        self.detected_wsl_distros = detect_wsl_distros();
        if self.wsl_distro.is_empty() {
            if let Some(first) = self.detected_wsl_distros.first() {
                self.wsl_distro = first.clone();
            }
        }
        self.picker_rx = None;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.picker_rx = None;
    }

    /// Poll pending async folder selection from Windows COM dialog.
    pub fn poll_picker(&mut self) {
        if let Some(rx) = &self.picker_rx {
            if let Ok(result) = rx.try_recv() {
                if let Some(chosen_path) = result {
                    self.path_input = chosen_path.to_string_lossy().to_string();
                    if !self.name_manually_edited {
                        if let Some(folder_name) = chosen_path.file_name().and_then(|n| n.to_str())
                        {
                            if !folder_name.trim().is_empty() {
                                self.name = folder_name.to_string();
                            }
                        }
                    }
                }
                self.picker_rx = None;
            }
        }
    }

    /// Render the New Workspace modal. Returns `Some(NewWorkspaceRequest)` upon submission.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        allowed_root: Option<&Path>,
    ) -> Option<NewWorkspaceRequest> {
        if !self.is_open {
            return None;
        }
        self.poll_picker();

        let mut submitted = None;
        let mut should_close = false;

        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.close();
            return None;
        }

        let extra_h = if allowed_root.is_some() { 26.0 } else { 0.0 };
        let modal_h = match self.backend_tab {
            BackendTab::Local => 268.0 + extra_h,
            BackendTab::Wsl => 300.0 + extra_h,
            BackendTab::Ssh => 320.0 + extra_h,
        };
        let screen = centered_modal_pos(ctx, 500.0, modal_h);
        egui::Window::new("new_workspace_modal")
            .fixed_pos(screen)
            .fixed_size(Vec2::new(500.0, modal_h))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .frame(modal_frame())
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Create New Workspace")
                            .size(15.0)
                            .color(Color32::WHITE)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new(
                            "Choose a Local Windows folder, WSL Linux distro, or OpenSSH host.",
                        )
                        .size(11.5)
                        .color(Color32::from_rgb(155, 165, 185)),
                    );
                    if let Some(root) = allowed_root {
                        ui.add_space(4.0);
                        ui.label(icon_label(
                            UiIcon::Lock,
                            &format!(
                                "Folder Lock Active — local workspaces must stay inside: {}",
                                display_clean_path(root)
                            ),
                            11.0,
                            Some(Color32::from_rgb(125, 225, 165)),
                        ));
                    }
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut self.backend_tab,
                            BackendTab::Local,
                            icon_text(UiIcon::Laptop, "Local Folder"),
                        );
                        ui.selectable_value(
                            &mut self.backend_tab,
                            BackendTab::Wsl,
                            icon_text(UiIcon::Terminal, "WSL Linux"),
                        );
                        ui.selectable_value(
                            &mut self.backend_tab,
                            BackendTab::Ssh,
                            icon_text(UiIcon::Cloud, "Remote SSH"),
                        );
                    });
                    ui.separator();
                    ui.add_space(6.0);

                    let mut local_path_error: Option<String> = None;

                    match self.backend_tab {
                        BackendTab::Local => {
                            ui.label(
                                egui::RichText::new("Project Folder")
                                    .size(12.0)
                                    .color(Color32::from_rgb(200, 210, 230)),
                            );
                            ui.horizontal(|ui| {
                                let edit = egui::TextEdit::singleline(&mut self.path_input)
                                    .desired_width(355.0)
                                    .text_color(Color32::WHITE);
                                ui.add(edit);

                                let is_browsing = self.picker_rx.is_some();
                                let btn_text = if is_browsing { "Choosing..." } else { "Browse..." };
                                if ui.add_enabled(!is_browsing, egui::Button::new(btn_text)).clicked()
                                {
                                    let candidate = PathBuf::from(self.path_input.trim());
                                    let initial = allowed_root
                                        .filter(|r| {
                                            validate_workspace_path(&candidate, Some(r)).is_err()
                                        })
                                        .map(Path::to_path_buf)
                                        .unwrap_or(candidate);
                                    self.picker_rx =
                                        Some(spawn_folder_picker_async(Some(initial), ctx.clone()));
                                }
                            });
                            if let Err(err) = validate_workspace_path(
                                &PathBuf::from(self.path_input.trim()),
                                allowed_root,
                            ) {
                                local_path_error = Some(err.to_string());
                            }
                            if let Some(ref err_msg) = local_path_error {
                                ui.label(icon_label(
                                    UiIcon::AlertTriangle,
                                    err_msg,
                                    11.0,
                                    Some(Color32::from_rgb(245, 110, 110)),
                                ));
                            }
                        }
                        BackendTab::Wsl => {
                            if !self.detected_wsl_distros.is_empty() {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label("Detected:");
                                    for d in &self.detected_wsl_distros {
                                        if ui
                                            .selectable_label(self.wsl_distro == *d, d.as_str())
                                            .clicked()
                                        {
                                            self.wsl_distro = d.clone();
                                            if !self.name_manually_edited {
                                                self.name = format!("WSL ({})", d);
                                            }
                                        }
                                    }
                                });
                            }
                            ui.horizontal(|ui| {
                                ui.label("Distro (optional):");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.wsl_distro)
                                        .desired_width(180.0)
                                        .hint_text("Default distro"),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label("Linux Path:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.wsl_linux_cwd)
                                        .desired_width(280.0)
                                        .hint_text("~ or /home/user/repo"),
                                );
                            });
                        }
                        BackendTab::Ssh => {
                            ui.horizontal(|ui| {
                                ui.label("SSH Target:");
                                let resp = ui.add(
                                    egui::TextEdit::singleline(&mut self.ssh_target)
                                        .desired_width(240.0)
                                        .hint_text("user@host or ~/.ssh/config alias"),
                                );
                                if resp.changed() && !self.name_manually_edited {
                                    let t = self.ssh_target.trim();
                                    if !t.is_empty() {
                                        self.name = format!("SSH: {t}");
                                    }
                                }
                                ui.label("Port:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.ssh_port)
                                        .desired_width(55.0)
                                        .hint_text("22"),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label("Remote Dir:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.ssh_remote_dir)
                                        .desired_width(280.0)
                                        .hint_text("~ or /srv/app"),
                                );
                            });
                            ui.label(
                                egui::RichText::new("Uses Windows OpenSSH (ssh.exe / ssh-agent); zero passwords stored by Age Max.")
                                    .size(10.5)
                                    .color(Color32::from_rgb(145, 155, 175)),
                            );
                        }
                    }

                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("Workspace Name")
                            .size(12.0)
                            .color(Color32::from_rgb(200, 210, 230)),
                    );
                    let name_edit = egui::TextEdit::singleline(&mut self.name)
                        .desired_width(ui.available_width())
                        .text_color(Color32::WHITE);
                    if ui.add(name_edit).changed() {
                        self.name_manually_edited = true;
                    }

                    ui.add_space(6.0);
                    ui.checkbox(
                        &mut self.launch_agy,
                        "Start Antigravity Agent (agy) inside this workspace immediately",
                    );

                    ui.add_space(10.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let valid_name = !self.name.trim().is_empty();
                        let valid_backend = match self.backend_tab {
                            BackendTab::Local => local_path_error.is_none(),
                            BackendTab::Wsl => true,
                            BackendTab::Ssh => !self.ssh_target.trim().is_empty(),
                        };
                        if ui
                            .add_enabled(
                                valid_name && valid_backend,
                                egui::Button::new("Create Workspace"),
                            )
                            .clicked()
                            || (valid_name
                                && valid_backend
                                && ui.input(|i| i.key_pressed(Key::Enter)))
                        {
                            let backend = match self.backend_tab {
                                BackendTab::Local => WorkspaceBackend::Local,
                                BackendTab::Wsl => WorkspaceBackend::Wsl {
                                    distro: {
                                        let d = self.wsl_distro.trim();
                                        if d.is_empty() { None } else { Some(d.to_string()) }
                                    },
                                    linux_cwd: {
                                        let c = self.wsl_linux_cwd.trim();
                                        if c.is_empty() { None } else { Some(c.to_string()) }
                                    },
                                },
                                BackendTab::Ssh => WorkspaceBackend::Ssh {
                                    target: self.ssh_target.trim().to_string(),
                                    port: self.ssh_port.trim().parse::<u16>().ok(),
                                    remote_dir: {
                                        let c = self.ssh_remote_dir.trim();
                                        if c.is_empty() { None } else { Some(c.to_string()) }
                                    },
                                },
                            };
                            let fallback_cwd = std::env::current_dir()
                                .unwrap_or_else(|_| PathBuf::from("."));
                            let cwd = if self.backend_tab == BackendTab::Local {
                                PathBuf::from(self.path_input.trim())
                            } else {
                                fallback_cwd
                            };
                            submitted = Some(NewWorkspaceRequest {
                                name: self.name.trim().to_string(),
                                cwd,
                                launch_agy: self.launch_agy,
                                backend,
                            });
                            should_close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            should_close = true;
                        }
                    });
                });
            });

        if should_close {
            self.close();
        }
        submitted
    }
}

/// Modal state for renaming a workspace or tab.
#[derive(Debug, Default)]
pub struct RenameModal {
    pub is_open: bool,
    pub target: Option<RenameTarget>,
    pub draft: String,
    focus_requested: bool,
}

impl RenameModal {
    pub fn open(&mut self, target: RenameTarget, current_name: &str) {
        self.is_open = true;
        self.target = Some(target);
        self.draft = current_name.to_string();
        self.focus_requested = false;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.target = None;
        self.draft.clear();
        self.focus_requested = false;
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<(RenameTarget, String)> {
        if !self.is_open {
            return None;
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.close();
            return None;
        }

        let target = self.target?;
        let title = match target {
            RenameTarget::Workspace(_) => "Rename Workspace",
            RenameTarget::Tab { .. } => "Rename Tab",
        };

        let mut submitted = None;
        let mut should_close = false;
        let pos = centered_modal_pos(ctx, 360.0, 120.0);

        egui::Window::new("rename_item_modal")
            .fixed_pos(pos)
            .fixed_size(Vec2::new(360.0, 115.0))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .frame(modal_frame())
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(title)
                            .size(14.0)
                            .color(Color32::WHITE)
                            .strong(),
                    );
                    ui.add_space(6.0);
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.draft)
                            .desired_width(ui.available_width())
                            .text_color(Color32::WHITE),
                    );
                    if !self.focus_requested {
                        resp.request_focus();
                        self.focus_requested = true;
                    }
                    ui.add_space(10.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let valid = !self.draft.trim().is_empty();
                        if ui.add_enabled(valid, egui::Button::new("Save")).clicked()
                            || (valid && ui.input(|i| i.key_pressed(Key::Enter)))
                        {
                            submitted = Some((target, self.draft.trim().to_string()));
                            should_close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            should_close = true;
                        }
                    });
                });
            });

        if should_close {
            self.close();
        }
        submitted
    }
}

/// Render the top header strip with app brand mark and centered dynamic Command Palette search pill.
pub fn render_header_strip(
    ui: &mut egui::Ui,
    config: &AgeMaxConfig,
    resolved_theme: &ResolvedTheme,
) -> Option<ToolbarAction> {
    if !config.general.effective_show_toolbar() {
        return None;
    }

    let tokens = UiTokens::from_theme(resolved_theme);
    let mut action = None;

    let (bar_rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), metrics::HEADER_HEIGHT),
        egui::Sense::hover(),
    );
    let painter = ui.painter();
    painter.rect_filled(bar_rect, 0.0, tokens.bg_chrome);
    painter.line_segment(
        [
            Pos2::new(bar_rect.min.x, bar_rect.max.y),
            Pos2::new(bar_rect.max.x, bar_rect.max.y),
        ],
        Stroke::new(1.0, tokens.border_subtle),
    );

    // Left brand mark: purple rounded badge + "Age Max"
    let logo_rect = Rect::from_min_size(
        Pos2::new(bar_rect.min.x + 12.0, bar_rect.center().y - 11.0),
        Vec2::new(22.0, 22.0),
    );
    painter.rect_filled(logo_rect, metrics::RADIUS_SM, tokens.accent_primary);
    let lc = logo_rect.center();
    painter.line_segment(
        [
            Pos2::new(lc.x - 4.5, lc.y + 5.0),
            Pos2::new(lc.x, lc.y - 5.5),
        ],
        Stroke::new(2.0, Color32::WHITE),
    );
    painter.line_segment(
        [
            Pos2::new(lc.x, lc.y - 5.5),
            Pos2::new(lc.x + 4.5, lc.y + 5.0),
        ],
        Stroke::new(2.0, Color32::WHITE),
    );
    painter.line_segment(
        [
            Pos2::new(lc.x - 2.6, lc.y + 1.5),
            Pos2::new(lc.x + 2.6, lc.y + 1.5),
        ],
        Stroke::new(1.7, Color32::WHITE),
    );
    painter.text(
        Pos2::new(logo_rect.max.x + 8.0, bar_rect.center().y),
        egui::Align2::LEFT_CENTER,
        "Age Max",
        egui::FontId::proportional(13.5),
        tokens.text_primary,
    );

    // Centered clickable search pill showing active Command Palette chord dynamically
    let pill_w = (bar_rect.width() - 280.0).clamp(240.0, 440.0);
    let pill_rect = Rect::from_center_size(bar_rect.center(), Vec2::new(pill_w, 26.0));
    let pill_resp = ui.interact(
        pill_rect,
        egui::Id::unique("top_search_pill"),
        egui::Sense::click(),
    );
    let hovered = pill_resp.hovered();
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    painter.rect_filled(
        pill_rect,
        metrics::RADIUS_MD,
        if hovered {
            tokens.bg_elevated
        } else {
            tokens.bg_surface
        },
    );
    painter.rect_stroke(
        pill_rect,
        metrics::RADIUS_MD,
        Stroke::new(
            1.0,
            if hovered {
                tokens.border_focus
            } else {
                tokens.border_subtle
            },
        ),
        egui::StrokeKind::Inside,
    );

    let shortcut_label = config.keybindings.command_palette.trim();
    let placeholder_rect = pill_rect.shrink2(Vec2::new(10.0, 2.0));
    let mut job = egui::text::LayoutJob::default();
    job.append(
        UiIcon::Search.as_str(),
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::new(12.5, crate::icons::lucide_font_family()),
            color: tokens.text_muted,
            valign: egui::Align::Center,
            ..Default::default()
        },
    );
    job.append(
        "  Search workspaces, run action... ",
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::proportional(12.0),
            color: tokens.text_muted,
            valign: egui::Align::Center,
            ..Default::default()
        },
    );
    let galley = painter.layout_job(job);
    painter.galley(
        Pos2::new(
            placeholder_rect.min.x,
            placeholder_rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        tokens.text_muted,
    );
    if !shortcut_label.is_empty() {
        painter.text(
            Pos2::new(placeholder_rect.max.x, placeholder_rect.center().y),
            egui::Align2::RIGHT_CENTER,
            shortcut_label,
            egui::FontId::proportional(11.0),
            tokens.text_muted,
        );
    }

    if pill_resp.clicked() {
        action = Some(ToolbarAction::OpenCommandPalette);
    }

    action
}

/// Render the Guided Mode top utility toolbar above the tab strip.
pub fn render_top_toolbar(
    ui: &mut egui::Ui,
    config: &AgeMaxConfig,
    resolved_theme: &ResolvedTheme,
    is_sidebar_collapsed: bool,
    unread_count: usize,
) -> Option<ToolbarAction> {
    if !config.general.effective_show_toolbar() {
        return None;
    }

    let mut action = None;
    let show_hints = config.general.effective_show_shortcut_hints();
    let kb = &config.keybindings;
    let chrome = ThemeChromeColors::from_theme(resolved_theme);

    let frame = egui::Frame::new()
        .fill(chrome.panel_bg)
        .stroke(Stroke::new(1.0, chrome.panel_stroke))
        .inner_margin(Vec2::new(8.0, 5.0));

    frame.show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            let sidebar_btn = ui.button(icon_text(
                UiIcon::Menu,
                if is_sidebar_collapsed {
                    "Show Sidebar"
                } else {
                    "Sidebar"
                },
            ));
            if hint_tooltip(sidebar_btn, show_hints, &kb.toggle_sidebar).clicked() {
                action = Some(ToolbarAction::ToggleSidebar);
            }

            if hint_tooltip(
                ui.button(icon_text(UiIcon::Plus, "Workspace")),
                show_hints,
                "Choose a project folder",
            )
            .clicked()
            {
                action = Some(ToolbarAction::OpenNewWorkspace);
            }

            if hint_tooltip(
                ui.button(icon_text(UiIcon::Plus, "Tab")),
                show_hints,
                &kb.new_tab,
            )
            .clicked()
            {
                action = Some(ToolbarAction::NewTab);
            }

            ui.separator();

            if hint_tooltip(
                ui.button(icon_text(UiIcon::Columns, "Split Right")),
                show_hints,
                &kb.split_horizontal,
            )
            .clicked()
            {
                action = Some(ToolbarAction::SplitRight);
            }

            if hint_tooltip(
                ui.button(icon_text(UiIcon::Rows, "Split Down")),
                show_hints,
                &kb.split_vertical,
            )
            .clicked()
            {
                action = Some(ToolbarAction::SplitDown);
            }

            let agy_btn = ui.button(icon_label(
                UiIcon::Play,
                "Start agy",
                12.5,
                Some(Color32::from_rgb(125, 225, 165)),
            ));
            if hint_tooltip(
                agy_btn,
                show_hints,
                "Launch Antigravity CLI in the active pane",
            )
            .clicked()
            {
                action = Some(ToolbarAction::LaunchAgy);
            }

            if hint_tooltip(
                ui.button(icon_text(UiIcon::Globe, "Preview")),
                show_hints,
                "Open an embedded WebView2 browser preview split pane",
            )
            .clicked()
            {
                action = Some(ToolbarAction::OpenBrowserPane);
            }

            if hint_tooltip(
                ui.button(icon_text(UiIcon::Bot, "ACP")),
                show_hints,
                "Switch active pane to live Antigravity ACP visual card panel",
            )
            .clicked()
            {
                action = Some(ToolbarAction::OpenAcpPanel);
            }

            if hint_tooltip(
                ui.button(icon_text(UiIcon::Users, "Team")),
                show_hints,
                "Create a coordinated multi-agent team split grid tab",
            )
            .clicked()
            {
                action = Some(ToolbarAction::LaunchTeamGrid);
            }

            if hint_tooltip(
                ui.button(icon_text(UiIcon::LayoutGrid, "Grid")),
                show_hints,
                "Mission Control overview grid of all live sessions",
            )
            .clicked()
            {
                action = Some(ToolbarAction::ToggleGridView);
            }

            if hint_tooltip(
                ui.button(icon_text(UiIcon::Megaphone, "Broadcast")),
                show_hints,
                "Send a confirmed prompt to selected live sessions",
            )
            .clicked()
            {
                action = Some(ToolbarAction::OpenBroadcast);
            }

            if hint_tooltip(
                ui.button(icon_text(UiIcon::Home, "Home")),
                show_hints,
                "Open Welcome launcher & recent sessions",
            )
            .clicked()
            {
                action = Some(ToolbarAction::OpenWelcome);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let target_mode = match config.general.ui_mode {
                    UiMode::Guided => UiMode::Minimal,
                    UiMode::Minimal => UiMode::Guided,
                };
                let mode_text = match config.general.ui_mode {
                    UiMode::Guided => "View: Guided",
                    UiMode::Minimal => "View: Minimal",
                };
                if hint_tooltip(
                    ui.button(mode_text),
                    show_hints,
                    "Switch between Guided buttons and Minimal terminal chrome",
                )
                .clicked()
                {
                    action = Some(ToolbarAction::SwitchUiMode(target_mode));
                }

                if hint_tooltip(
                    ui.button(icon_text(UiIcon::Settings, "Settings")),
                    show_hints,
                    "Open visual Settings, UI View Mode & Shortcuts",
                )
                .clicked()
                {
                    action = Some(ToolbarAction::OpenViewSettings);
                }

                let alert_label = if unread_count > 0 {
                    icon_label(
                        UiIcon::Bell,
                        &format!("Alerts ({unread_count})"),
                        12.5,
                        Some(Color32::from_rgb(245, 175, 45)),
                    )
                } else {
                    icon_label(
                        UiIcon::Bell,
                        "Alerts",
                        12.5,
                        Some(Color32::from_rgb(185, 190, 205)),
                    )
                };
                if hint_tooltip(ui.button(alert_label), show_hints, &kb.toggle_notifications)
                    .clicked()
                {
                    action = Some(ToolbarAction::ToggleNotifications);
                }

                if hint_tooltip(
                    ui.button(icon_text(UiIcon::Command, "Actions")),
                    show_hints,
                    &kb.command_palette,
                )
                .clicked()
                {
                    action = Some(ToolbarAction::OpenCommandPalette);
                }
            });
        });
    });

    action
}

/// Properties passed into `render_pane_header`.
pub struct PaneHeaderProps<'a> {
    pub header_rect: Rect,
    pub pane_id: PaneId,
    pub status: AgentStatus,
    pub is_focused: bool,
    pub is_zoomed: bool,
    pub show_hints: bool,
    pub show_mascot: bool,
    pub resolved_theme: &'a ResolvedTheme,
    pub keybindings: &'a KeybindingsConfig,
    pub team_role: Option<(&'a str, bool)>,
}

/// Render a slim 26px per-pane action header bar in Guided mode.
pub fn render_pane_header(
    ui: &mut egui::Ui,
    props: PaneHeaderProps<'_>,
) -> Option<PaneHeaderAction> {
    let mut action = None;
    let header_rect = props.header_rect;
    let pane_id = props.pane_id;
    let kb = props.keybindings;
    let chrome = ThemeChromeColors::from_theme(props.resolved_theme);

    let bg = if props.is_focused {
        chrome.panel_alt_bg
    } else {
        chrome.panel_bg
    };
    ui.painter().rect_filled(header_rect, 0.0, bg);
    ui.painter().line_segment(
        [
            Pos2::new(header_rect.min.x, header_rect.max.y),
            Pos2::new(header_rect.max.x, header_rect.max.y),
        ],
        Stroke::new(1.0, chrome.panel_stroke),
    );

    let padded = header_rect.shrink2(Vec2::new(6.0, 2.0));
    ui.scope_builder(
        egui::UiBuilder::new()
            .id_salt(("pane_header", pane_id.0))
            .max_rect(padded),
        |ui| {
            ui.horizontal(|ui| {
                if props.show_mascot {
                    let (mascot_rect, _) =
                        ui.allocate_exact_size(Vec2::new(20.0, 18.0), egui::Sense::hover());
                    let time_secs = ui.input(|i| i.time);
                    render_agent_mascot(
                        ui.painter(),
                        mascot_rect,
                        props.status,
                        time_secs,
                        props.resolved_theme,
                    );
                }
                let (status_label, status_color) = friendly_status_badge(props.status);
                ui.label(icon_label(
                    status_icon(props.status),
                    status_label,
                    11.0,
                    Some(status_color),
                ));
                if let Some((role, is_coord)) = props.team_role {
                    let role_color = if is_coord {
                        Color32::from_rgb(130, 165, 255)
                    } else {
                        Color32::from_rgb(115, 215, 185)
                    };
                    ui.label(icon_label(UiIcon::Users, role, 10.5, Some(role_color)));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if hint_tooltip(
                        ui.small_button(icon_rich(UiIcon::X, 11.0, None)),
                        props.show_hints,
                        &kb.close_pane,
                    )
                    .clicked()
                    {
                        action = Some(PaneHeaderAction::Close(pane_id));
                    }
                    let zoom_icon = if props.is_zoomed {
                        UiIcon::Minimize
                    } else {
                        UiIcon::Maximize
                    };
                    if hint_tooltip(
                        ui.small_button(icon_rich(zoom_icon, 11.0, None)),
                        props.show_hints,
                        &kb.zoom_pane,
                    )
                    .clicked()
                    {
                        action = Some(PaneHeaderAction::ToggleZoom(pane_id));
                    }
                    if hint_tooltip(
                        ui.small_button(icon_rich(UiIcon::Rows, 11.0, None)),
                        props.show_hints,
                        &kb.split_vertical,
                    )
                    .clicked()
                    {
                        action = Some(PaneHeaderAction::SplitDown(pane_id));
                    }
                    if hint_tooltip(
                        ui.small_button(icon_rich(UiIcon::Columns, 11.0, None)),
                        props.show_hints,
                        &kb.split_horizontal,
                    )
                    .clicked()
                    {
                        action = Some(PaneHeaderAction::SplitRight(pane_id));
                    }
                    if hint_tooltip(
                        ui.small_button(icon_small(UiIcon::Play, "agy")),
                        props.show_hints,
                        "Launch Antigravity CLI in this pane",
                    )
                    .clicked()
                    {
                        action = Some(PaneHeaderAction::LaunchAgy(pane_id));
                    }
                });
            });
        },
    );

    action
}

fn friendly_status_badge(status: AgentStatus) -> (&'static str, Color32) {
    match status {
        AgentStatus::Idle => ("Ready", Color32::from_rgb(145, 155, 175)),
        AgentStatus::Working => ("Working…", Color32::from_rgb(105, 175, 255)),
        AgentStatus::Thinking => ("Thinking…", Color32::from_rgb(115, 200, 255)),
        AgentStatus::RunningTool => ("Running Tool…", Color32::from_rgb(245, 195, 85)),
        AgentStatus::WaitingForInput => ("Needs Answer", Color32::from_rgb(245, 175, 45)),
        AgentStatus::Done => ("Done", Color32::from_rgb(45, 200, 110)),
        AgentStatus::Failed => ("Error", Color32::from_rgb(235, 60, 60)),
    }
}

fn hint_tooltip(response: egui::Response, show_hints: bool, hint: &str) -> egui::Response {
    if show_hints && !hint.trim().is_empty() {
        response.on_hover_text(hint)
    } else {
        response
    }
}

/// Render a clear working-directory & folder-lock status bar so users always see where terminals/agents run (`REQ-080`).
pub fn render_workspace_folder_bar(
    ui: &mut egui::Ui,
    cwd: &Path,
    backend: &WorkspaceBackend,
    allowed_root: Option<&Path>,
    sandbox_enabled: bool,
    resolved_theme: &ResolvedTheme,
) -> Option<ToolbarAction> {
    let mut action = None;
    let chrome = ThemeChromeColors::from_theme(resolved_theme);
    let frame = egui::Frame::new()
        .fill(chrome.window_bg)
        .stroke(Stroke::new(1.0, chrome.panel_stroke))
        .inner_margin(Vec2::new(8.0, 3.0));

    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            let location = match backend {
                WorkspaceBackend::Local => display_clean_path(cwd),
                WorkspaceBackend::Wsl { distro, linux_cwd } => format!(
                    "WSL {} ({})",
                    distro.as_deref().unwrap_or("default"),
                    linux_cwd.as_deref().unwrap_or("~")
                ),
                WorkspaceBackend::Ssh {
                    target, remote_dir, ..
                } => format!("SSH {} ({})", target, remote_dir.as_deref().unwrap_or("~")),
            };

            ui.label(icon_label(
                UiIcon::Folder,
                "Working in:",
                11.0,
                Some(Color32::from_rgb(150, 160, 180)),
            ));
            ui.label(
                egui::RichText::new(location)
                    .size(11.0)
                    .color(Color32::from_rgb(215, 225, 242)),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let lock_btn = if let Some(root) = allowed_root {
                    let txt = if sandbox_enabled {
                        format!("Locked: {} + --sandbox", display_clean_path(root))
                    } else {
                        format!("Locked inside: {}", display_clean_path(root))
                    };
                    ui.small_button(icon_label(
                        UiIcon::Lock,
                        &txt,
                        10.5,
                        Some(Color32::from_rgb(125, 225, 165)),
                    ))
                    .on_hover_text("Click to manage Safe Folder Lock & agy sandbox settings")
                } else {
                    let (ic, txt) = if sandbox_enabled {
                        (UiIcon::Shield, "Sandbox On • Unrestricted")
                    } else {
                        (UiIcon::LockOpen, "Unrestricted Folder (Click to Lock…)")
                    };
                    ui.small_button(icon_label(
                        ic,
                        txt,
                        10.5,
                        Some(Color32::from_rgb(245, 190, 95)),
                    ))
                    .on_hover_text("Restrict where Age Max & agents can open workspaces")
                };

                if lock_btn.clicked() {
                    action = Some(ToolbarAction::OpenSafetySettings);
                }
            });
        });
    });

    action
}

pub(crate) fn centered_modal_pos(ctx: &egui::Context, width: f32, height: f32) -> Pos2 {
    let screen = ctx
        .input(|i| i.raw.screen_rect)
        .unwrap_or(Rect::from_min_size(Pos2::ZERO, Vec2::new(1150.0, 720.0)));
    Pos2::new(
        ((screen.width() - width) * 0.5).max(16.0),
        ((screen.height() - height) * 0.35).max(24.0),
    )
}

pub(crate) fn modal_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgb(24, 27, 38))
        .corner_radius(8.0)
        .stroke(Stroke::new(1.5, Color32::from_rgb(70, 110, 200)))
        .inner_margin(14.0)
}
