use crate::icons::{icon_rich, UiIcon};
use crate::visual_alive::ThemeChromeColors;
use agemax_config::theme::ResolvedTheme;
use agemax_core::workspace::Tab;
use eframe::egui::{self, Color32, Vec2};

pub enum TabBarAction {
    Select(usize),
    Create,
    Rename(usize),
    Close(usize),
}

pub fn render_tab_bar(
    ui: &mut egui::Ui,
    tabs: &[Tab],
    active_index: usize,
    show_hints: bool,
    theme: &ResolvedTheme,
) -> Option<TabBarAction> {
    let mut action = None;
    let chrome = ThemeChromeColors::from_theme(theme);

    let frame = egui::Frame::new().fill(chrome.panel_bg).inner_margin(4.0);

    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            for (idx, tab) in tabs.iter().enumerate() {
                let is_active = idx == active_index;
                let bg_color = if is_active {
                    chrome.panel_alt_bg
                } else {
                    chrome.panel_bg
                };

                let tab_frame = egui::Frame::new()
                    .fill(bg_color)
                    .corner_radius(3.0)
                    .inner_margin(Vec2::new(8.0, 4.0));

                tab_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let text_color = if is_active {
                            chrome.text_primary
                        } else {
                            chrome.text_muted
                        };

                        let link =
                            ui.link(egui::RichText::new(&tab.title).color(text_color).size(12.0));
                        let link = if show_hints {
                            link.on_hover_text("Click to focus • Double-click to rename")
                        } else {
                            link
                        };
                        if link.double_clicked() {
                            action = Some(TabBarAction::Rename(idx));
                        } else if link.clicked() {
                            action = Some(TabBarAction::Select(idx));
                        }

                        if is_active {
                            ui.add_space(2.0);
                            if ui
                                .small_button(icon_rich(UiIcon::Pencil, 10.5, Some(Color32::GRAY)))
                                .on_hover_text("Rename tab")
                                .clicked()
                            {
                                action = Some(TabBarAction::Rename(idx));
                            }
                        }

                        if tabs.len() > 1 {
                            ui.add_space(4.0);
                            if ui
                                .button(icon_rich(UiIcon::X, 11.0, Some(Color32::GRAY)))
                                .clicked()
                            {
                                action = Some(TabBarAction::Close(idx));
                            }
                        }
                    });
                });

                ui.add_space(2.0);
            }

            let add_btn = ui.button(icon_rich(UiIcon::Plus, 12.0, None));
            let add_btn = if show_hints {
                add_btn.on_hover_text("New Tab")
            } else {
                add_btn
            };
            if add_btn.clicked() {
                action = Some(TabBarAction::Create);
            }
        });
    });

    action
}
