use crate::folder_picker::spawn_folder_picker_async;
use crate::icons::{icon_label, icon_rich, icon_text, UiIcon};
use crate::ui_tokens::{metrics, UiTokens};
use crate::visual_alive::render_agent_mascot;
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

/// Render the Guided Mode single-row grouped icon toolbar with overflow menu and `View: Minimal | Full`.
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
    let tokens = UiTokens::from_theme(resolved_theme);
    let compact = ui.available_width() < 710.0;

    let frame = egui::Frame::new()
        .fill(tokens.bg_chrome)
        .inner_margin(Vec2::new(10.0, 6.0));

    let resp = frame.show(ui, |ui| {
        ui.set_min_height(metrics::TOOLBAR_HEIGHT - 12.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;

            // Group 1: Workspace & Sidebar
            let sidebar_hint = if kb.toggle_sidebar.trim().is_empty() {
                if is_sidebar_collapsed {
                    "Show Sidebar"
                } else {
                    "Hide Sidebar"
                }
                .to_string()
            } else {
                format!(
                    "{} ({})",
                    if is_sidebar_collapsed {
                        "Show Sidebar"
                    } else {
                        "Toggle Sidebar"
                    },
                    kb.toggle_sidebar
                )
            };
            if toolbar_icon_btn(
                ui,
                UiIcon::Menu,
                !is_sidebar_collapsed,
                &tokens,
                show_hints,
                &sidebar_hint,
            )
            .clicked()
            {
                action = Some(ToolbarAction::ToggleSidebar);
            }

            if toolbar_icon_btn(
                ui,
                UiIcon::Plus,
                false,
                &tokens,
                show_hints,
                "New Workspace",
            )
            .clicked()
            {
                action = Some(ToolbarAction::OpenNewWorkspace);
            }

            toolbar_divider(ui, &tokens);

            // Group 2: Layout splits & Grid overview
            if toolbar_icon_btn(
                ui,
                UiIcon::Columns,
                false,
                &tokens,
                show_hints,
                &format!("Split Right ({})", kb.split_horizontal),
            )
            .clicked()
            {
                action = Some(ToolbarAction::SplitRight);
            }

            if toolbar_icon_btn(
                ui,
                UiIcon::Rows,
                false,
                &tokens,
                show_hints,
                &format!("Split Down ({})", kb.split_vertical),
            )
            .clicked()
            {
                action = Some(ToolbarAction::SplitDown);
            }

            if toolbar_icon_btn(
                ui,
                UiIcon::LayoutGrid,
                false,
                &tokens,
                show_hints,
                "All Agents Mission Grid",
            )
            .clicked()
            {
                action = Some(ToolbarAction::ToggleGridView);
            }

            toolbar_divider(ui, &tokens);

            // Primary filled CTA pill
            let agy_btn = ui.add(
                egui::Button::new(icon_label(
                    UiIcon::Play,
                    "Start agy",
                    12.5,
                    Some(Color32::WHITE),
                ))
                .fill(tokens.accent_primary)
                .stroke(Stroke::new(1.0, tokens.accent_hover))
                .corner_radius(metrics::RADIUS_SM)
                .min_size(Vec2::new(92.0, metrics::ICON_BTN_SIZE.y)),
            );
            if hint_tooltip(
                agy_btn,
                show_hints,
                "Launch Antigravity CLI in the active pane",
            )
            .clicked()
            {
                action = Some(ToolbarAction::LaunchAgy);
            }

            toolbar_divider(ui, &tokens);

            // Group 3: Agent / View tools (secondary actions collapse into overflow on tight widths)
            if toolbar_icon_btn(
                ui,
                UiIcon::Eye,
                false,
                &tokens,
                show_hints,
                "Preview: open embedded WebView2 browser pane",
            )
            .clicked()
            {
                action = Some(ToolbarAction::OpenBrowserPane);
            }

            if toolbar_icon_btn(
                ui,
                UiIcon::Cpu,
                false,
                &tokens,
                show_hints,
                "ACP: switch active pane to Antigravity ACP panel",
            )
            .clicked()
            {
                action = Some(ToolbarAction::OpenAcpPanel);
            }

            if toolbar_icon_btn(
                ui,
                UiIcon::Users,
                false,
                &tokens,
                show_hints,
                "Team: create multi-agent Coordinator + Workers tab",
            )
            .clicked()
            {
                action = Some(ToolbarAction::LaunchTeamGrid);
            }

            if !compact {
                if toolbar_icon_btn(
                    ui,
                    UiIcon::Radio,
                    false,
                    &tokens,
                    show_hints,
                    "Broadcast prompt to selected sessions",
                )
                .clicked()
                {
                    action = Some(ToolbarAction::OpenBroadcast);
                }

                if toolbar_icon_btn(
                    ui,
                    UiIcon::Home,
                    false,
                    &tokens,
                    show_hints,
                    "Home: Welcome launcher & recent workspaces",
                )
                .clicked()
                {
                    action = Some(ToolbarAction::OpenWelcome);
                }
            }

            // Overflow menu (`...`) preserves access to all actions at >= 900 px widths
            let overflow_btn = ui
                .add(
                    egui::Button::new(icon_rich(
                        UiIcon::MoreHorizontal,
                        14.0,
                        Some(tokens.text_muted),
                    ))
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::NONE)
                    .corner_radius(metrics::RADIUS_SM)
                    .min_size(Vec2::new(24.0, metrics::ICON_BTN_SIZE.y)),
                )
                .on_hover_text("More toolbar actions");
            egui::Popup::menu(&overflow_btn).show(|ui| {
                if ui.button(icon_text(UiIcon::Plus, "New Tab")).clicked() {
                    action = Some(ToolbarAction::NewTab);
                    ui.close();
                }
                if compact {
                    if ui
                        .button(icon_text(UiIcon::Radio, "Broadcast Prompt"))
                        .clicked()
                    {
                        action = Some(ToolbarAction::OpenBroadcast);
                        ui.close();
                    }
                    if ui
                        .button(icon_text(UiIcon::Home, "Welcome / Home"))
                        .clicked()
                    {
                        action = Some(ToolbarAction::OpenWelcome);
                        ui.close();
                    }
                }
                if ui
                    .button(icon_text(UiIcon::Command, "Command Palette / Actions"))
                    .clicked()
                {
                    action = Some(ToolbarAction::OpenCommandPalette);
                    ui.close();
                }
                if ui.button(icon_text(UiIcon::Settings, "Settings")).clicked() {
                    action = Some(ToolbarAction::OpenViewSettings);
                    ui.close();
                }
            });

            // Right utility cluster: View segmented pill, Zap/Actions, Bell+badge
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;

                // View: [Minimal | Full] segmented toggle
                let current_mode = config.general.ui_mode;
                egui::Frame::new()
                    .fill(tokens.bg_surface)
                    .stroke(Stroke::new(1.0, tokens.border_subtle))
                    .corner_radius(metrics::RADIUS_SM)
                    .inner_margin(Vec2::new(2.0, 2.0))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        let full_active = current_mode == UiMode::Guided;
                        let full_btn = ui.add(
                            egui::Button::new(egui::RichText::new("Full").size(11.0).color(
                                if full_active {
                                    Color32::WHITE
                                } else {
                                    tokens.text_muted
                                },
                            ))
                            .fill(if full_active {
                                tokens.accent_primary
                            } else {
                                Color32::TRANSPARENT
                            })
                            .stroke(Stroke::NONE)
                            .corner_radius(4.0)
                            .min_size(Vec2::new(40.0, 20.0)),
                        );
                        if hint_tooltip(
                            full_btn,
                            show_hints,
                            "Full mode: show toolbars and pane headers",
                        )
                        .clicked()
                            && !full_active
                        {
                            action = Some(ToolbarAction::SwitchUiMode(UiMode::Guided));
                        }

                        let min_active = current_mode == UiMode::Minimal;
                        let min_btn = ui.add(
                            egui::Button::new(egui::RichText::new("Minimal").size(11.0).color(
                                if min_active {
                                    Color32::WHITE
                                } else {
                                    tokens.text_muted
                                },
                            ))
                            .fill(if min_active {
                                tokens.accent_primary
                            } else {
                                Color32::TRANSPARENT
                            })
                            .stroke(Stroke::NONE)
                            .corner_radius(4.0)
                            .min_size(Vec2::new(52.0, 20.0)),
                        );
                        if hint_tooltip(
                            min_btn,
                            show_hints,
                            "Minimal mode: hide chrome and focus purely on terminals",
                        )
                        .clicked()
                            && !min_active
                        {
                            action = Some(ToolbarAction::SwitchUiMode(UiMode::Minimal));
                        }
                    });

                ui.label(
                    egui::RichText::new("View:")
                        .size(11.0)
                        .color(tokens.text_muted),
                );

                if !compact
                    && toolbar_icon_btn(
                        ui,
                        UiIcon::Settings,
                        false,
                        &tokens,
                        show_hints,
                        "Open visual Settings, View Mode & Shortcuts",
                    )
                    .clicked()
                {
                    action = Some(ToolbarAction::OpenViewSettings);
                }

                // Quick Actions (Command Palette)
                if toolbar_icon_btn(
                    ui,
                    UiIcon::Zap,
                    false,
                    &tokens,
                    show_hints,
                    &format!("Actions ({})", kb.command_palette),
                )
                .clicked()
                {
                    action = Some(ToolbarAction::OpenCommandPalette);
                }

                // Alerts Bell + circular unread badge
                let bell_hint = if unread_count > 0 {
                    format!(
                        "Alerts: {unread_count} unread ({})",
                        kb.toggle_notifications
                    )
                } else {
                    format!("Alerts & Activity Feed ({})", kb.toggle_notifications)
                };
                let bell_resp = toolbar_icon_btn(
                    ui,
                    UiIcon::Bell,
                    unread_count > 0,
                    &tokens,
                    show_hints,
                    &bell_hint,
                );
                if unread_count > 0 {
                    let badge_center =
                        Pos2::new(bell_resp.rect.max.x - 3.0, bell_resp.rect.min.y + 4.0);
                    ui.painter()
                        .circle_filled(badge_center, 6.5, tokens.alert_badge_bg);
                    let count_str = if unread_count > 9 {
                        "9+".to_string()
                    } else {
                        unread_count.to_string()
                    };
                    ui.painter().text(
                        badge_center,
                        egui::Align2::CENTER_CENTER,
                        count_str,
                        egui::FontId::proportional(9.0),
                        Color32::WHITE,
                    );
                }
                if bell_resp.clicked() {
                    action = Some(ToolbarAction::ToggleNotifications);
                }
            });
        });
    });

    let outer = resp.response.rect;
    ui.painter().line_segment(
        [
            Pos2::new(outer.min.x, outer.max.y),
            Pos2::new(outer.max.x, outer.max.y),
        ],
        Stroke::new(1.0, tokens.border_subtle),
    );

    action
}

fn toolbar_icon_btn(
    ui: &mut egui::Ui,
    icon: UiIcon,
    highlighted: bool,
    tokens: &UiTokens,
    show_hints: bool,
    hint: &str,
) -> egui::Response {
    let btn = egui::Button::new(icon_rich(
        icon,
        14.0,
        Some(if highlighted {
            tokens.text_primary
        } else {
            tokens.text_muted
        }),
    ))
    .fill(if highlighted {
        tokens.bg_elevated
    } else {
        tokens.bg_surface
    })
    .stroke(Stroke::new(
        1.0,
        if highlighted {
            tokens.border_focus
        } else {
            tokens.border_subtle
        },
    ))
    .corner_radius(metrics::RADIUS_SM)
    .min_size(metrics::ICON_BTN_SIZE);

    hint_tooltip(ui.add(btn), show_hints, hint)
}

fn toolbar_divider(ui: &mut egui::Ui, tokens: &UiTokens) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(5.0, 18.0), egui::Sense::hover());
    ui.painter().line_segment(
        [
            Pos2::new(rect.center().x, rect.min.y + 1.0),
            Pos2::new(rect.center().x, rect.max.y - 1.0),
        ],
        Stroke::new(1.0, tokens.border_subtle),
    );
}

/// Properties passed into `render_pane_header`.
pub struct PaneHeaderProps<'a> {
    pub header_rect: Rect,
    pub pane_id: PaneId,
    pub pane_label: &'a str,
    pub status: AgentStatus,
    pub is_focused: bool,
    pub is_zoomed: bool,
    pub show_hints: bool,
    pub show_mascot: bool,
    pub resolved_theme: &'a ResolvedTheme,
    pub keybindings: &'a KeybindingsConfig,
    pub team_role: Option<(&'a str, bool)>,
}

/// Render a clean 28px per-pane action header bar with agent label, status capsule, and icon actions.
pub fn render_pane_header(
    ui: &mut egui::Ui,
    props: PaneHeaderProps<'_>,
) -> Option<PaneHeaderAction> {
    let mut action = None;
    let header_rect = props.header_rect;
    let pane_id = props.pane_id;
    let kb = props.keybindings;
    let tokens = UiTokens::from_theme(props.resolved_theme);

    let bg = if props.is_focused {
        tokens.bg_elevated
    } else {
        tokens.bg_surface
    };
    ui.painter().rect_filled(
        header_rect,
        egui::CornerRadius {
            nw: metrics::RADIUS_SM as u8,
            ne: metrics::RADIUS_SM as u8,
            sw: 0,
            se: 0,
        },
        bg,
    );
    ui.painter().line_segment(
        [
            Pos2::new(header_rect.min.x, header_rect.max.y),
            Pos2::new(header_rect.max.x, header_rect.max.y),
        ],
        Stroke::new(1.0, tokens.border_subtle),
    );

    let padded = header_rect.shrink2(Vec2::new(8.0, 3.0));
    ui.scope_builder(
        egui::UiBuilder::new()
            .id_salt(("pane_header", pane_id.0))
            .max_rect(padded),
        |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;

                if props.show_mascot {
                    let (mascot_rect, _) =
                        ui.allocate_exact_size(Vec2::new(18.0, 16.0), egui::Sense::hover());
                    let time_secs = ui.input(|i| i.time);
                    render_agent_mascot(
                        ui.painter(),
                        mascot_rect,
                        props.status,
                        time_secs,
                        props.resolved_theme,
                    );
                }

                ui.label(icon_rich(
                    UiIcon::Terminal,
                    12.0,
                    Some(if props.is_focused {
                        tokens.text_primary
                    } else {
                        tokens.text_muted
                    }),
                ));
                ui.label(
                    egui::RichText::new(props.pane_label)
                        .size(12.0)
                        .color(if props.is_focused {
                            tokens.text_primary
                        } else {
                            tokens.text_muted
                        })
                        .strong(),
                );

                // Rounded status capsule pill matching the mockup (`Running`, `Needs input`, `Done`, etc.)
                let (status_label, pill_fg, pill_bg, dot_col) =
                    pane_status_pill(props.status, &tokens);
                egui::Frame::new()
                    .fill(pill_bg)
                    .stroke(Stroke::new(
                        0.8,
                        Color32::from_rgba_unmultiplied(pill_fg.r(), pill_fg.g(), pill_fg.b(), 80),
                    ))
                    .corner_radius(10.0)
                    .inner_margin(Vec2::new(7.0, 1.5))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            let (drect, _) =
                                ui.allocate_exact_size(Vec2::new(6.0, 10.0), egui::Sense::hover());
                            ui.painter().circle_filled(drect.center(), 2.8, dot_col);
                            ui.label(
                                egui::RichText::new(status_label)
                                    .size(10.5)
                                    .color(pill_fg)
                                    .strong(),
                            );
                        });
                    });

                if let Some((role, is_coord)) = props.team_role {
                    let role_color = if is_coord {
                        Color32::from_rgb(130, 165, 255)
                    } else {
                        Color32::from_rgb(115, 215, 185)
                    };
                    ui.label(icon_label(UiIcon::Users, role, 10.5, Some(role_color)));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;

                    if pane_header_icon_btn(
                        ui,
                        UiIcon::X,
                        &tokens,
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
                    if pane_header_icon_btn(ui, zoom_icon, &tokens, props.show_hints, &kb.zoom_pane)
                        .clicked()
                    {
                        action = Some(PaneHeaderAction::ToggleZoom(pane_id));
                    }
                    if pane_header_icon_btn(
                        ui,
                        UiIcon::Rows,
                        &tokens,
                        props.show_hints,
                        &kb.split_vertical,
                    )
                    .clicked()
                    {
                        action = Some(PaneHeaderAction::SplitDown(pane_id));
                    }
                    if pane_header_icon_btn(
                        ui,
                        UiIcon::Columns,
                        &tokens,
                        props.show_hints,
                        &kb.split_horizontal,
                    )
                    .clicked()
                    {
                        action = Some(PaneHeaderAction::SplitRight(pane_id));
                    }
                    if pane_header_icon_btn(
                        ui,
                        UiIcon::Play,
                        &tokens,
                        props.show_hints,
                        "Start agy in this pane",
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

fn pane_header_icon_btn(
    ui: &mut egui::Ui,
    icon: UiIcon,
    tokens: &UiTokens,
    show_hints: bool,
    hint: &str,
) -> egui::Response {
    let btn = egui::Button::new(icon_rich(icon, 11.5, Some(tokens.text_muted)))
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::NONE)
        .corner_radius(4.0)
        .min_size(Vec2::new(20.0, 20.0));
    hint_tooltip(ui.add(btn), show_hints, hint)
}

fn pane_status_pill(
    status: AgentStatus,
    tokens: &UiTokens,
) -> (&'static str, Color32, Color32, Color32) {
    match status {
        AgentStatus::Idle => (
            "Ready",
            tokens.text_muted,
            tokens.bg_surface,
            Color32::from_rgb(100, 116, 139),
        ),
        AgentStatus::Working | AgentStatus::Thinking => (
            "Running",
            tokens.status_running_fg,
            tokens.status_running_bg,
            Color32::from_rgb(34, 197, 94),
        ),
        AgentStatus::RunningTool => (
            "Running tool",
            tokens.status_running_fg,
            tokens.status_running_bg,
            Color32::from_rgb(34, 197, 94),
        ),
        AgentStatus::WaitingForInput => (
            "Needs input",
            tokens.status_waiting_fg,
            tokens.status_waiting_bg,
            Color32::from_rgb(245, 158, 11),
        ),
        AgentStatus::Done => (
            "Done",
            tokens.status_done_fg,
            tokens.bg_surface,
            Color32::from_rgb(100, 116, 139),
        ),
        AgentStatus::Failed => (
            "Error",
            tokens.status_error_fg,
            tokens.status_error_bg,
            Color32::from_rgb(239, 68, 68),
        ),
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
    let tokens = UiTokens::from_theme(resolved_theme);
    let frame = egui::Frame::new()
        .fill(tokens.bg_canvas)
        .inner_margin(Vec2::new(10.0, 5.0));

    let resp = frame.show(ui, |ui| {
        ui.set_min_size(Vec2::new(
            ui.available_width(),
            metrics::PATH_BAR_HEIGHT - 10.0,
        ));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

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

            ui.label(icon_rich(UiIcon::Folder, 13.0, Some(tokens.text_muted)));
            ui.add(
                egui::Label::new(
                    egui::RichText::new(location)
                        .size(11.5)
                        .color(tokens.text_muted),
                )
                .truncate(),
            );

            ui.add_space(4.0);

            let lock_btn = if let Some(root) = allowed_root {
                let txt = if sandbox_enabled {
                    "Locked inside folder • --sandbox"
                } else {
                    "Locked inside folder"
                };
                ui.add(
                    egui::Button::new(icon_label(UiIcon::Lock, txt, 11.0, Some(tokens.lock_fg)))
                        .fill(tokens.lock_fill)
                        .stroke(Stroke::new(1.0, tokens.lock_border))
                        .corner_radius(metrics::RADIUS_PILL)
                        .min_size(Vec2::new(0.0, 21.0)),
                )
                .on_hover_text(format!(
                    "Safe Folder Lock active: {}\nClick to manage folder restrictions & --sandbox",
                    display_clean_path(root)
                ))
            } else {
                let (ic, txt) = if sandbox_enabled {
                    (UiIcon::Shield, "Sandbox On • Unrestricted")
                } else {
                    (UiIcon::LockOpen, "Unrestricted Folder (Click to Lock…)")
                };
                ui.add(
                    egui::Button::new(icon_label(ic, txt, 11.0, Some(tokens.unlocked_fg)))
                        .fill(tokens.unlocked_fill)
                        .stroke(Stroke::new(1.0, tokens.unlocked_border))
                        .corner_radius(metrics::RADIUS_PILL)
                        .min_size(Vec2::new(0.0, 21.0)),
                )
                .on_hover_text("Restrict where Age Max & agents can open workspaces")
            };

            if lock_btn.clicked() {
                action = Some(ToolbarAction::OpenSafetySettings);
            }
        });
    });

    let outer = resp.response.rect;
    ui.painter().line_segment(
        [
            Pos2::new(outer.min.x, outer.max.y),
            Pos2::new(outer.max.x, outer.max.y),
        ],
        Stroke::new(1.0, tokens.border_subtle),
    );

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
