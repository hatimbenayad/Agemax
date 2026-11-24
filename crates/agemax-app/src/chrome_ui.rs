use crate::folder_picker::spawn_folder_picker_async;
pub use agemax_config::keybindings::KeybindingsConfig;
use agemax_config::schema::{AgeMaxConfig, UiMode};
use agemax_core::layout::PaneId;
use agemax_core::status::AgentStatus;
use crossbeam_channel::Receiver;
use eframe::egui::{self, Color32, Key, Pos2, Rect, Stroke, Vec2};
use std::path::PathBuf;

/// Actions emitted by the Guided mode Top Utility Toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarAction {
    ToggleSidebar,
    OpenNewWorkspace,
    NewTab,
    SplitRight,
    SplitDown,
    LaunchAgy,
    OpenCommandPalette,
    ToggleNotifications,
    OpenViewSettings,
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

/// Modal state for picking a project folder and creating a named Workspace.
#[derive(Debug)]
pub struct NewWorkspaceModal {
    pub is_open: bool,
    pub name: String,
    pub path_input: String,
    pub name_manually_edited: bool,
    pub launch_agy: bool,
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
            picker_rx: None,
        }
    }

    pub fn open(&mut self, default_dir: &std::path::Path, default_index: usize) {
        self.is_open = true;
        self.path_input = default_dir.to_string_lossy().to_string();
        self.name = default_dir
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|s| !s.trim().is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("Workspace {}", default_index));
        self.name_manually_edited = false;
        self.launch_agy = false;
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

    /// Render the New Workspace modal. Returns `Some((name, path, launch_agy))` upon submission.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<(String, PathBuf, bool)> {
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

        let screen = centered_modal_pos(ctx, 480.0, 220.0);
        egui::Window::new("new_workspace_modal")
            .fixed_pos(screen)
            .fixed_size(Vec2::new(480.0, 215.0))
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
                            "Pick a project folder and give this workspace a clear name.",
                        )
                        .size(11.5)
                        .color(Color32::from_rgb(155, 165, 185)),
                    );
                    ui.add_space(10.0);

                    ui.label(
                        egui::RichText::new("Project Folder")
                            .size(12.0)
                            .color(Color32::from_rgb(200, 210, 230)),
                    );
                    ui.horizontal(|ui| {
                        let edit = egui::TextEdit::singleline(&mut self.path_input)
                            .desired_width(340.0)
                            .text_color(Color32::WHITE);
                        ui.add(edit);

                        let is_browsing = self.picker_rx.is_some();
                        let btn_text = if is_browsing {
                            "Choosing..."
                        } else {
                            "Browse..."
                        };
                        if ui
                            .add_enabled(!is_browsing, egui::Button::new(btn_text))
                            .clicked()
                        {
                            let initial = PathBuf::from(self.path_input.trim());
                            self.picker_rx =
                                Some(spawn_folder_picker_async(Some(initial), ctx.clone()));
                        }
                    });

                    ui.add_space(8.0);
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

                    ui.add_space(8.0);
                    ui.checkbox(
                        &mut self.launch_agy,
                        "Start Antigravity Agent (agy) inside this workspace immediately",
                    );

                    ui.add_space(12.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let valid_name = !self.name.trim().is_empty();
                        let valid_path = !self.path_input.trim().is_empty();
                        if ui
                            .add_enabled(
                                valid_name && valid_path,
                                egui::Button::new("Create Workspace"),
                            )
                            .clicked()
                            || (valid_name && valid_path && ui.input(|i| i.key_pressed(Key::Enter)))
                        {
                            submitted = Some((
                                self.name.trim().to_string(),
                                PathBuf::from(self.path_input.trim()),
                                self.launch_agy,
                            ));
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

/// Render the Guided Mode top utility toolbar above the tab strip.
pub fn render_top_toolbar(
    ui: &mut egui::Ui,
    config: &AgeMaxConfig,
    is_sidebar_collapsed: bool,
    unread_count: usize,
) -> Option<ToolbarAction> {
    if !config.general.effective_show_toolbar() {
        return None;
    }

    let mut action = None;
    let show_hints = config.general.effective_show_shortcut_hints();
    let kb = &config.keybindings;

    let frame = egui::Frame::new()
        .fill(Color32::from_rgb(20, 23, 33))
        .stroke(Stroke::new(1.0, Color32::from_rgb(35, 39, 52)))
        .inner_margin(Vec2::new(8.0, 5.0));

    frame.show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            let sidebar_btn = ui.button(if is_sidebar_collapsed {
                "☰ Show Sidebar"
            } else {
                "☰ Sidebar"
            });
            if hint_tooltip(sidebar_btn, show_hints, &kb.toggle_sidebar).clicked() {
                action = Some(ToolbarAction::ToggleSidebar);
            }

            if hint_tooltip(
                ui.button("+ Workspace"),
                show_hints,
                "Choose a project folder",
            )
            .clicked()
            {
                action = Some(ToolbarAction::OpenNewWorkspace);
            }

            if hint_tooltip(ui.button("+ Tab"), show_hints, &kb.new_tab).clicked() {
                action = Some(ToolbarAction::NewTab);
            }

            ui.separator();

            if hint_tooltip(ui.button("⬌ Split Right"), show_hints, &kb.split_horizontal).clicked()
            {
                action = Some(ToolbarAction::SplitRight);
            }

            if hint_tooltip(ui.button("⬍ Split Down"), show_hints, &kb.split_vertical).clicked() {
                action = Some(ToolbarAction::SplitDown);
            }

            let agy_btn = ui.button(
                egui::RichText::new("▶ Start agy")
                    .color(Color32::from_rgb(125, 225, 165))
                    .strong(),
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
                    ui.button("⚙ Settings"),
                    show_hints,
                    "Open visual Settings, UI View Mode & Shortcuts",
                )
                .clicked()
                {
                    action = Some(ToolbarAction::OpenViewSettings);
                }

                let alert_label = if unread_count > 0 {
                    egui::RichText::new(format!("🔔 Alerts ({})", unread_count))
                        .color(Color32::from_rgb(245, 175, 45))
                        .strong()
                } else {
                    egui::RichText::new("🔔 Alerts").color(Color32::from_rgb(185, 190, 205))
                };
                if hint_tooltip(ui.button(alert_label), show_hints, &kb.toggle_notifications)
                    .clicked()
                {
                    action = Some(ToolbarAction::ToggleNotifications);
                }

                if hint_tooltip(ui.button("⌘ Actions"), show_hints, &kb.command_palette).clicked()
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
    pub keybindings: &'a KeybindingsConfig,
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

    let bg = if props.is_focused {
        Color32::from_rgb(27, 32, 46)
    } else {
        Color32::from_rgb(21, 24, 34)
    };
    ui.painter().rect_filled(header_rect, 0.0, bg);
    ui.painter().line_segment(
        [
            Pos2::new(header_rect.min.x, header_rect.max.y),
            Pos2::new(header_rect.max.x, header_rect.max.y),
        ],
        Stroke::new(1.0, Color32::from_rgb(38, 43, 58)),
    );

    let padded = header_rect.shrink2(Vec2::new(6.0, 2.0));
    ui.scope_builder(
        egui::UiBuilder::new()
            .id_salt(("pane_header", pane_id.0))
            .max_rect(padded),
        |ui| {
            ui.horizontal(|ui| {
                let (status_label, status_color) = friendly_status_badge(props.status);
                ui.label(
                    egui::RichText::new(status_label)
                        .size(11.0)
                        .color(status_color)
                        .strong(),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if hint_tooltip(ui.small_button("×"), props.show_hints, &kb.close_pane)
                        .clicked()
                    {
                        action = Some(PaneHeaderAction::Close(pane_id));
                    }
                    let zoom_text = if props.is_zoomed { "🗗" } else { "⛶" };
                    if hint_tooltip(ui.small_button(zoom_text), props.show_hints, &kb.zoom_pane)
                        .clicked()
                    {
                        action = Some(PaneHeaderAction::ToggleZoom(pane_id));
                    }
                    if hint_tooltip(ui.small_button("⬍"), props.show_hints, &kb.split_vertical)
                        .clicked()
                    {
                        action = Some(PaneHeaderAction::SplitDown(pane_id));
                    }
                    if hint_tooltip(ui.small_button("⬌"), props.show_hints, &kb.split_horizontal)
                        .clicked()
                    {
                        action = Some(PaneHeaderAction::SplitRight(pane_id));
                    }
                    if hint_tooltip(
                        ui.small_button("▶ agy"),
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
        AgentStatus::Idle => ("● Ready", Color32::from_rgb(145, 155, 175)),
        AgentStatus::Working => ("⟳ Working…", Color32::from_rgb(105, 175, 255)),
        AgentStatus::WaitingForInput => ("⚠ Needs Answer", Color32::from_rgb(245, 175, 45)),
        AgentStatus::Done => ("✓ Done", Color32::from_rgb(45, 200, 110)),
        AgentStatus::Failed => ("✕ Error", Color32::from_rgb(235, 60, 60)),
    }
}

fn hint_tooltip(response: egui::Response, show_hints: bool, hint: &str) -> egui::Response {
    if show_hints && !hint.trim().is_empty() {
        response.on_hover_text(hint)
    } else {
        response
    }
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
