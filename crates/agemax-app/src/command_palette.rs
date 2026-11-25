use crate::icons::{icon_rich, UiIcon};
use crate::ui_tokens::UiTokens;
use agemax_config::keybindings::KeybindingsConfig;
use agemax_config::schema::CustomCommand;
use agemax_config::theme::ResolvedTheme;
use agemax_core::layout::WorkspaceId;
use eframe::egui::{self, Color32, Key, Pos2, Rect, Sense, Stroke, Vec2};

#[derive(Debug, Clone)]
pub enum PaletteAction {
    SwitchWorkspace(WorkspaceId),
    NewTab,
    SplitRight,
    SplitDown,
    ClosePane,
    ToggleZoom,
    ToggleSidebar,
    JumpUnread,
    ToggleNotifications,
    NewWorkspace,
    LaunchAgy,
    OpenBrowser,
    OpenAcpPanel,
    LaunchTeamGrid,
    ToggleGridView,
    OpenBroadcast,
    OpenWelcome,
    ToggleUiMode,
    OpenViewSettings,
    Custom(CustomCommand),
}

#[derive(Debug, Clone)]
pub struct PaletteWorkspaceItem {
    pub id: WorkspaceId,
    pub name: String,
    pub cwd_display: String,
    pub git_branch: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct PaletteItem {
    pub title: String,
    pub description: Option<String>,
    pub category: &'static str,
    pub action: PaletteAction,
}

#[derive(Debug)]
pub struct CommandPalette {
    pub is_open: bool,
    pub query: String,
    pub selected_index: usize,
    items: Vec<PaletteItem>,
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandPalette {
    pub fn new() -> Self {
        let mut palette = Self {
            is_open: false,
            query: String::new(),
            selected_index: 0,
            items: Vec::new(),
        };
        palette.sync_builtins(&KeybindingsConfig::default());
        palette
    }

    /// Refresh built-in items using the active keybindings while preserving workspaces and custom commands.
    pub fn sync_builtins(&mut self, kb: &KeybindingsConfig) {
        let workspaces: Vec<PaletteItem> = self
            .items
            .iter()
            .filter(|i| matches!(i.action, PaletteAction::SwitchWorkspace(_)))
            .cloned()
            .collect();
        let custom: Vec<PaletteItem> = self
            .items
            .iter()
            .filter(|i| matches!(i.action, PaletteAction::Custom(_)))
            .cloned()
            .collect();

        let mut items = workspaces;
        items.extend(vec![
            PaletteItem {
                title: "Start Antigravity Agent (agy)".to_string(),
                description: Some("Launch agy in active pane".to_string()),
                category: "Agent",
                action: PaletteAction::LaunchAgy,
            },
            PaletteItem {
                title: "Open Antigravity ACP Panel".to_string(),
                description: Some("Structured tool/artifact card view via agy acp".to_string()),
                category: "Agent",
                action: PaletteAction::OpenAcpPanel,
            },
            PaletteItem {
                title: "Launch Multi-Agent Team Grid".to_string(),
                description: Some("Create Coordinator + Worker team layout".to_string()),
                category: "Agent",
                action: PaletteAction::LaunchTeamGrid,
            },
            PaletteItem {
                title: "Toggle All-Agents Mission Grid".to_string(),
                description: Some("Inspect all live sessions across workspaces".to_string()),
                category: "View",
                action: PaletteAction::ToggleGridView,
            },
            PaletteItem {
                title: "Broadcast Prompt to Selected Sessions".to_string(),
                description: Some("Confirmed multi-pane prompt dispatch".to_string()),
                category: "Agent",
                action: PaletteAction::OpenBroadcast,
            },
            PaletteItem {
                title: "Open Welcome / Empty State Launcher".to_string(),
                description: Some("Quick-start agent presets & recent sessions".to_string()),
                category: "View",
                action: PaletteAction::OpenWelcome,
            },
            PaletteItem {
                title: "Open WebView2 Browser Preview".to_string(),
                description: Some("Embedded browser preview pane".to_string()),
                category: "Browser",
                action: PaletteAction::OpenBrowser,
            },
            PaletteItem {
                title: "Create New Workspace".to_string(),
                description: Some("Pick folder & name".to_string()),
                category: "Workspace",
                action: PaletteAction::NewWorkspace,
            },
            PaletteItem {
                title: "New Terminal Tab".to_string(),
                description: Some(kb.new_tab.clone()),
                category: "Tab",
                action: PaletteAction::NewTab,
            },
            PaletteItem {
                title: "Split Pane Right".to_string(),
                description: Some(kb.split_horizontal.clone()),
                category: "Pane",
                action: PaletteAction::SplitRight,
            },
            PaletteItem {
                title: "Split Pane Down".to_string(),
                description: Some(kb.split_vertical.clone()),
                category: "Pane",
                action: PaletteAction::SplitDown,
            },
            PaletteItem {
                title: "Close Focused Pane".to_string(),
                description: Some(kb.close_pane.clone()),
                category: "Pane",
                action: PaletteAction::ClosePane,
            },
            PaletteItem {
                title: "Toggle Zoom Pane".to_string(),
                description: Some(kb.zoom_pane.clone()),
                category: "Pane",
                action: PaletteAction::ToggleZoom,
            },
            PaletteItem {
                title: "Toggle Workspace Sidebar".to_string(),
                description: Some(kb.toggle_sidebar.clone()),
                category: "View",
                action: PaletteAction::ToggleSidebar,
            },
            PaletteItem {
                title: "Switch UI Mode (Guided / Minimal)".to_string(),
                description: Some("Toggle toolbar & pane buttons".to_string()),
                category: "View",
                action: PaletteAction::ToggleUiMode,
            },
            PaletteItem {
                title: "Open View & Shortcuts Settings".to_string(),
                description: None,
                category: "View",
                action: PaletteAction::OpenViewSettings,
            },
            PaletteItem {
                title: "Jump to Oldest Unread Waiting Pane".to_string(),
                description: Some(kb.jump_unread.clone()),
                category: "Attention",
                action: PaletteAction::JumpUnread,
            },
            PaletteItem {
                title: "Toggle Notification Panel".to_string(),
                description: Some(kb.toggle_notifications.clone()),
                category: "Attention",
                action: PaletteAction::ToggleNotifications,
            },
        ]);

        items.extend(custom);
        self.items = items;
    }

    /// Sync active workspace jump entries at the top of the palette without disturbing actions.
    pub fn sync_workspaces(&mut self, workspaces: &[PaletteWorkspaceItem]) {
        let non_workspace: Vec<PaletteItem> = self
            .items
            .iter()
            .filter(|i| !matches!(i.action, PaletteAction::SwitchWorkspace(_)))
            .cloned()
            .collect();

        let mut items: Vec<PaletteItem> = workspaces
            .iter()
            .map(|ws| {
                let title = if ws.is_active {
                    format!("{} (active)", ws.name)
                } else {
                    ws.name.clone()
                };
                let description = match ws.git_branch.as_deref() {
                    Some(branch) if !branch.is_empty() => {
                        Some(format!("{branch} · {}", ws.cwd_display))
                    }
                    _ => Some(ws.cwd_display.clone()),
                };
                PaletteItem {
                    title,
                    description,
                    category: "Workspace",
                    action: PaletteAction::SwitchWorkspace(ws.id),
                }
            })
            .collect();

        items.extend(non_workspace);
        self.items = items;
    }

    pub fn set_custom_commands(&mut self, commands: &[CustomCommand]) {
        // Keep built-in actions, remove old custom commands, then append new ones
        self.items
            .retain(|i| !matches!(i.action, PaletteAction::Custom(_)));
        for cmd in commands {
            self.items.push(PaletteItem {
                title: cmd.name.clone(),
                description: cmd.description.clone(),
                category: "Custom",
                action: PaletteAction::Custom(cmd.clone()),
            });
        }
    }

    pub fn open(&mut self) {
        self.is_open = true;
        self.query.clear();
        self.selected_index = 0;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.query.clear();
        self.selected_index = 0;
    }

    pub fn filtered_indices(&self) -> Vec<usize> {
        let q = self.query.trim().to_lowercase();
        if q.is_empty() {
            (0..self.items.len()).collect()
        } else {
            self.items
                .iter()
                .enumerate()
                .filter(|(_, i)| {
                    i.title.to_lowercase().contains(&q)
                        || i.category.to_lowercase().contains(&q)
                        || i.description
                            .as_ref()
                            .map(|d| d.to_lowercase().contains(&q))
                            .unwrap_or(false)
                })
                .map(|(idx, _)| idx)
                .collect()
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, theme: &ResolvedTheme) -> Option<PaletteAction> {
        if !self.is_open {
            return None;
        }

        let tokens = UiTokens::from_theme(theme);
        let mut executed_action = None;

        let screen_rect = ctx
            .input(|i| i.raw.screen_rect)
            .unwrap_or(Rect::from_min_size(Pos2::ZERO, Vec2::new(1150.0, 720.0)));
        let modal_width = 540.0_f32.min(screen_rect.width() - 40.0);
        let modal_x = (screen_rect.width() - modal_width) / 2.0;
        let modal_y = 64.0;

        egui::Window::new("command_palette_modal")
            .fixed_pos(Pos2::new(modal_x, modal_y))
            .fixed_size(Vec2::new(modal_width, 360.0))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .frame(
                egui::Frame::new()
                    .fill(tokens.bg_surface)
                    .corner_radius(10.0)
                    .stroke(Stroke::new(1.5, tokens.border_focus))
                    .inner_margin(12.0),
            )
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // Search input bar
                    ui.horizontal(|ui| {
                        ui.label(icon_rich(UiIcon::Search, 14.0, Some(tokens.text_muted)));
                        let text_edit = egui::TextEdit::singleline(&mut self.query)
                            .hint_text("Search workspaces, run action...")
                            .text_color(tokens.text_primary)
                            .desired_width(ui.available_width() - 20.0);
                        let response = ui.add(text_edit);
                        response.request_focus();
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Filtered indices
                    let filtered_indices = self.filtered_indices();
                    let count = filtered_indices.len();
                    if count > 0 && self.selected_index >= count {
                        self.selected_index = count - 1;
                    } else if count == 0 {
                        self.selected_index = 0;
                    }

                    // Keyboard navigation in palette
                    if ui.input(|i| i.key_pressed(Key::Escape)) {
                        self.close();
                        return;
                    }

                    if count > 0 {
                        if ui.input(|i| i.key_pressed(Key::ArrowDown)) {
                            self.selected_index = (self.selected_index + 1) % count;
                        }
                        if ui.input(|i| i.key_pressed(Key::ArrowUp)) {
                            self.selected_index = if self.selected_index == 0 {
                                count - 1
                            } else {
                                self.selected_index - 1
                            };
                        }
                        if ui.input(|i| i.key_pressed(Key::Enter)) {
                            if let Some(&item_idx) = filtered_indices.get(self.selected_index) {
                                if let Some(item) = self.items.get(item_idx) {
                                    executed_action = Some(item.action.clone());
                                }
                                self.close();
                                return;
                            }
                        }
                    }

                    egui::ScrollArea::vertical()
                        .max_height(260.0)
                        .show(ui, |ui| {
                            for (list_pos, &item_idx) in filtered_indices.iter().enumerate() {
                                let item = &self.items[item_idx];
                                let is_selected = list_pos == self.selected_index;
                                let bg = if is_selected {
                                    tokens.bg_elevated
                                } else {
                                    Color32::TRANSPARENT
                                };

                                let frame = egui::Frame::new()
                                    .fill(bg)
                                    .corner_radius(6.0)
                                    .inner_margin(6.0);

                                let row_resp = frame
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            if matches!(
                                                item.action,
                                                PaletteAction::SwitchWorkspace(_)
                                            ) {
                                                ui.label(icon_rich(
                                                    UiIcon::Folder,
                                                    13.0,
                                                    Some(tokens.accent_primary),
                                                ));
                                            }
                                            ui.label(
                                                egui::RichText::new(&item.title)
                                                    .size(13.0)
                                                    .color(tokens.text_primary)
                                                    .strong(),
                                            );

                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if let Some(desc) = &item.description {
                                                        ui.label(
                                                            egui::RichText::new(desc)
                                                                .size(11.0)
                                                                .color(tokens.text_muted),
                                                        );
                                                    }
                                                    ui.label(
                                                        egui::RichText::new(format!(
                                                            "[{}]",
                                                            item.category
                                                        ))
                                                        .size(10.0)
                                                        .color(tokens.text_muted),
                                                    );
                                                },
                                            );
                                        });
                                    })
                                    .response
                                    .interact(Sense::click());

                                if row_resp.clicked() {
                                    executed_action = Some(item.action.clone());
                                }

                                ui.add_space(2.0);
                            }
                        });
                });
            });

        if executed_action.is_some() {
            self.close();
        }
        executed_action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_workspaces_preserves_builtins_and_custom_commands() {
        let mut palette = CommandPalette::new();
        palette.set_custom_commands(&[CustomCommand {
            name: "Run Cargo Check".to_string(),
            description: Some("Verify workspace".to_string()),
            command: "cargo check".to_string(),
            working_dir: None,
            keybinding: None,
        }]);

        let ws1 = WorkspaceId::new();
        let ws2 = WorkspaceId::new();
        palette.sync_workspaces(&[
            PaletteWorkspaceItem {
                id: ws1,
                name: "agemaxlab".to_string(),
                cwd_display: "C:\\src\\agemaxlab".to_string(),
                git_branch: Some("main".to_string()),
                is_active: true,
            },
            PaletteWorkspaceItem {
                id: ws2,
                name: "solarity".to_string(),
                cwd_display: "C:\\src\\solarity".to_string(),
                git_branch: None,
                is_active: false,
            },
        ]);

        palette.query = "solarity".to_string();
        let matches = palette.filtered_indices();
        assert_eq!(matches.len(), 1);
        assert!(matches!(
            palette.items[matches[0]].action,
            PaletteAction::SwitchWorkspace(id) if id == ws2
        ));

        palette.query = "Run Cargo".to_string();
        let custom_matches = palette.filtered_indices();
        assert_eq!(custom_matches.len(), 1);
        assert!(matches!(
            palette.items[custom_matches[0]].action,
            PaletteAction::Custom(_)
        ));

        palette.sync_builtins(&KeybindingsConfig::default());
        palette.query = "main".to_string();
        let branch_matches = palette.filtered_indices();
        assert!(branch_matches.iter().any(|&idx| matches!(
            palette.items[idx].action,
            PaletteAction::SwitchWorkspace(id) if id == ws1
        )));
    }
}
