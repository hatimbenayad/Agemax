use agemax_config::schema::CustomCommand;
use eframe::egui::{self, Color32, Key, Pos2, Rect, Stroke, Vec2};

#[derive(Debug, Clone)]
pub enum PaletteAction {
    SplitRight,
    SplitDown,
    ClosePane,
    ToggleZoom,
    ToggleSidebar,
    JumpUnread,
    ToggleNotifications,
    NewWorkspace,
    Custom(CustomCommand),
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
        let items = vec![
            PaletteItem {
                title: "Split Pane Right".to_string(),
                description: Some("Ctrl+Shift+D".to_string()),
                category: "Pane",
                action: PaletteAction::SplitRight,
            },
            PaletteItem {
                title: "Split Pane Down".to_string(),
                description: Some("Ctrl+Shift+R".to_string()),
                category: "Pane",
                action: PaletteAction::SplitDown,
            },
            PaletteItem {
                title: "Close Focused Pane".to_string(),
                description: Some("Ctrl+Shift+W".to_string()),
                category: "Pane",
                action: PaletteAction::ClosePane,
            },
            PaletteItem {
                title: "Toggle Zoom Pane".to_string(),
                description: Some("Ctrl+Shift+Enter".to_string()),
                category: "Pane",
                action: PaletteAction::ToggleZoom,
            },
            PaletteItem {
                title: "Toggle Workspace Sidebar".to_string(),
                description: Some("Ctrl+B".to_string()),
                category: "View",
                action: PaletteAction::ToggleSidebar,
            },
            PaletteItem {
                title: "Jump to Oldest Unread Waiting Pane".to_string(),
                description: Some("Ctrl+Shift+U".to_string()),
                category: "Attention",
                action: PaletteAction::JumpUnread,
            },
            PaletteItem {
                title: "Toggle Notification Panel".to_string(),
                description: Some("Ctrl+Shift+N".to_string()),
                category: "Attention",
                action: PaletteAction::ToggleNotifications,
            },
            PaletteItem {
                title: "Create New Workspace".to_string(),
                description: None,
                category: "Workspace",
                action: PaletteAction::NewWorkspace,
            },
        ];

        Self {
            is_open: false,
            query: String::new(),
            selected_index: 0,
            items,
        }
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

    pub fn show(&mut self, ctx: &egui::Context) -> Option<PaletteAction> {
        if !self.is_open {
            return None;
        }

        let mut executed_action = None;

        let screen_rect = ctx
            .input(|i| i.raw.screen_rect)
            .unwrap_or(Rect::from_min_size(Pos2::ZERO, Vec2::new(1150.0, 720.0)));
        let modal_width = 540.0_f32.min(screen_rect.width() - 40.0);
        let modal_x = (screen_rect.width() - modal_width) / 2.0;
        let modal_y = 70.0;

        egui::Window::new("command_palette_modal")
            .fixed_pos(Pos2::new(modal_x, modal_y))
            .fixed_size(Vec2::new(modal_width, 360.0))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(24, 27, 38))
                    .corner_radius(8.0)
                    .stroke(Stroke::new(1.5, Color32::from_rgb(70, 110, 200)))
                    .inner_margin(12.0),
            )
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // Search input bar
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("🔍")
                                .size(14.0)
                                .color(Color32::from_rgb(140, 145, 160)),
                        );
                        let text_edit = egui::TextEdit::singleline(&mut self.query)
                            .hint_text("Type a command or search actions...")
                            .text_color(Color32::WHITE)
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
                                    Color32::from_rgb(45, 55, 80)
                                } else {
                                    Color32::TRANSPARENT
                                };

                                let frame = egui::Frame::new()
                                    .fill(bg)
                                    .corner_radius(4.0)
                                    .inner_margin(6.0);

                                frame.show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(&item.title)
                                                .size(13.0)
                                                .color(Color32::from_rgb(230, 235, 245))
                                                .strong(),
                                        );

                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if let Some(desc) = &item.description {
                                                    ui.label(
                                                        egui::RichText::new(desc).size(11.0).color(
                                                            Color32::from_rgb(140, 150, 175),
                                                        ),
                                                    );
                                                }
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "[{}]",
                                                        item.category
                                                    ))
                                                    .size(10.0)
                                                    .color(Color32::from_rgb(100, 110, 130)),
                                                );
                                            },
                                        );
                                    });
                                });

                                ui.add_space(2.0);
                            }
                        });
                });
            });

        executed_action
    }
}
