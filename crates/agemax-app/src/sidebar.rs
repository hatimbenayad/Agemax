use agemax_core::workspace::Workspace;
use eframe::egui::{self, Color32, Stroke, Vec2};

pub struct SidebarProps<'a> {
    pub workspaces: &'a [Workspace],
    pub active_index: usize,
    pub is_collapsed: bool,
}

pub enum SidebarAction {
    SelectWorkspace(usize),
    CreateWorkspace,
    ToggleCollapse,
}

pub fn render_sidebar(ui: &mut egui::Ui, props: SidebarProps) -> Option<SidebarAction> {
    let mut action = None;
    let width = if props.is_collapsed { 48.0 } else { 220.0 };

    ui.allocate_ui(Vec2::new(width, ui.available_height()), |ui| {
        let frame = egui::Frame::new()
            .fill(Color32::from_rgb(20, 22, 30))
            .stroke(Stroke::new(1.0, Color32::from_rgb(35, 38, 50)));

        frame.show(ui, |ui| {
            ui.vertical(|ui| {
                // Header
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if !props.is_collapsed {
                        ui.heading(
                            egui::RichText::new("WORKSPACES")
                                .size(11.0)
                                .color(Color32::from_rgb(130, 135, 150))
                                .strong(),
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let toggle_label = if props.is_collapsed { "»" } else { "«" };
                        if ui.button(toggle_label).clicked() {
                            action = Some(SidebarAction::ToggleCollapse);
                        }
                    });
                });

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                // Workspace list
                for (idx, ws) in props.workspaces.iter().enumerate() {
                    let is_active = idx == props.active_index;
                    let bg_color = if is_active {
                        Color32::from_rgb(38, 42, 58)
                    } else {
                        Color32::TRANSPARENT
                    };

                    let btn_frame = egui::Frame::new()
                        .fill(bg_color)
                        .corner_radius(4.0)
                        .inner_margin(6.0);

                    btn_frame.show(ui, |ui| {
                        if props.is_collapsed {
                            let first_letter = ws.name.chars().next().unwrap_or('W').to_string();
                            let resp = ui.button(
                                egui::RichText::new(first_letter)
                                    .color(if is_active {
                                        Color32::WHITE
                                    } else {
                                        Color32::GRAY
                                    })
                                    .size(14.0)
                                    .strong(),
                            );
                            if resp.clicked() {
                                action = Some(SidebarAction::SelectWorkspace(idx));
                            }
                            resp.on_hover_text(&ws.name);
                        } else {
                            ui.horizontal(|ui| {
                                let label_text = egui::RichText::new(&ws.name)
                                    .color(if is_active {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_rgb(180, 185, 200)
                                    })
                                    .size(13.0)
                                    .strong();
                                if ui.link(label_text).clicked() {
                                    action = Some(SidebarAction::SelectWorkspace(idx));
                                }

                                if let Some(branch) = &ws.git_branch {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(format!(" {}", branch))
                                                    .size(10.0)
                                                    .color(Color32::from_rgb(110, 190, 140)),
                                            );
                                        },
                                    );
                                }
                            });

                            let folder_name = ws
                                .cwd
                                .file_name()
                                .map(|f| f.to_string_lossy().to_string())
                                .unwrap_or_else(|| ws.cwd.to_string_lossy().to_string());
                            ui.label(
                                egui::RichText::new(folder_name)
                                    .size(10.0)
                                    .color(Color32::from_rgb(110, 115, 130)),
                            );
                        }
                    });

                    ui.add_space(2.0);
                }

                // Add Workspace Button
                ui.add_space(8.0);
                if !props.is_collapsed {
                    if ui.button("+ New Workspace").clicked() {
                        action = Some(SidebarAction::CreateWorkspace);
                    }
                } else if ui.button("+").clicked() {
                    action = Some(SidebarAction::CreateWorkspace);
                }
            });
        });
    });

    action
}
