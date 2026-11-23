use agemax_core::model::TerminalSize;
use agemax_term::pty::{detect_default_shell, PtyConfig};
use agemax_term::surface::TerminalSurface;
use eframe::egui::{self, Color32, FontId, Key, Pos2, Rect, Vec2};
use std::time::Instant;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Age Max - Native Agent Multiplexer")
            .with_inner_size([1024.0, 680.0])
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };

    eframe::run_native(
        "AgeMax",
        native_options,
        Box::new(|cc| {
            configure_fonts(&cc.egui_ctx);
            Ok(Box::new(AgeMaxApp::new()))
        }),
    )
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "Cascadia Code".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(1, "Consolas".to_owned());
    ctx.set_fonts(fonts);
}

pub struct AgeMaxApp {
    surface: TerminalSurface,
    last_size: TerminalSize,
    font_size: f32,
    char_width: f32,
    char_height: f32,
    start_time: Instant,
    first_frame_rendered: bool,
}

impl AgeMaxApp {
    pub fn new() -> Self {
        let start_time = Instant::now();
        let initial_size = TerminalSize::new(120, 36);
        let shell_path = detect_default_shell();
        let config = PtyConfig {
            program: shell_path,
            args: Vec::new(),
            env: std::collections::HashMap::new(),
            cwd: None,
            cols: initial_size.cols,
            rows: initial_size.rows,
        };

        let surface = TerminalSurface::new(initial_size, config)
            .expect("Failed to initialize ConPTY terminal surface");

        Self {
            surface,
            last_size: initial_size,
            font_size: 14.0,
            char_width: 8.4,
            char_height: 18.0,
            start_time,
            first_frame_rendered: false,
        }
    }
}

impl Default for AgeMaxApp {
    fn default() -> Self {
        Self::new()
    }
}

impl AgeMaxApp {
    fn handle_keyboard_input(&mut self, ctx: &egui::Context) {
        let events = ctx.input(|i| i.events.clone());
        for event in events {
            match event {
                egui::Event::Text(text) => {
                    let _ = self.surface.send_input(text.as_bytes());
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    if modifiers.ctrl && !modifiers.shift && !modifiers.alt {
                        match key {
                            Key::C => {
                                let _ = self.surface.send_input(b"\x03");
                            }
                            Key::D => {
                                let _ = self.surface.send_input(b"\x04");
                            }
                            Key::Z => {
                                let _ = self.surface.send_input(b"\x1a");
                            }
                            Key::L => {
                                let _ = self.surface.send_input(b"\x0c");
                            }
                            Key::A => {
                                let _ = self.surface.send_input(b"\x01");
                            }
                            Key::E => {
                                let _ = self.surface.send_input(b"\x05");
                            }
                            Key::W => {
                                let _ = self.surface.send_input(b"\x17");
                            }
                            Key::U => {
                                let _ = self.surface.send_input(b"\x15");
                            }
                            Key::K => {
                                let _ = self.surface.send_input(b"\x0b");
                            }
                            _ => {}
                        }
                    } else if modifiers.ctrl && modifiers.shift {
                        if key == Key::V {
                            if let Some(clipboard) = ctx.input(|i| {
                                i.events.iter().find_map(|e| {
                                    if let egui::Event::Paste(text) = e {
                                        Some(text.clone())
                                    } else {
                                        None
                                    }
                                })
                            }) {
                                let _ = self.surface.send_input(clipboard.as_bytes());
                            }
                        }
                    } else if !modifiers.ctrl && !modifiers.alt {
                        match key {
                            Key::Enter => {
                                let _ = self.surface.send_input(b"\r");
                            }
                            Key::Backspace => {
                                let _ = self.surface.send_input(b"\x08");
                            }
                            Key::Tab => {
                                let _ = self.surface.send_input(b"\t");
                            }
                            Key::Escape => {
                                let _ = self.surface.send_input(b"\x1b");
                            }
                            Key::ArrowUp => {
                                let _ = self.surface.send_input(b"\x1b[A");
                            }
                            Key::ArrowDown => {
                                let _ = self.surface.send_input(b"\x1b[B");
                            }
                            Key::ArrowRight => {
                                let _ = self.surface.send_input(b"\x1b[C");
                            }
                            Key::ArrowLeft => {
                                let _ = self.surface.send_input(b"\x1b[D");
                            }
                            Key::Home => {
                                let _ = self.surface.send_input(b"\x1b[H");
                            }
                            Key::End => {
                                let _ = self.surface.send_input(b"\x1b[F");
                            }
                            Key::PageUp => {
                                let _ = self.surface.send_input(b"\x1b[5~");
                            }
                            Key::PageDown => {
                                let _ = self.surface.send_input(b"\x1b[6~");
                            }
                            _ => {}
                        }
                    }
                }
                egui::Event::Paste(text) => {
                    let _ = self.surface.send_input(text.as_bytes());
                }
                _ => {}
            }
        }
    }
}

impl eframe::App for AgeMaxApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.first_frame_rendered {
            let cold_start_ms = self.start_time.elapsed().as_millis();
            tracing::info!(
                "Age Max First Render: {} ms (Budget < 300 ms)",
                cold_start_ms
            );
            self.first_frame_rendered = true;
        }

        // Process incoming PTY data
        let has_new_data = self.surface.process_output();
        if has_new_data {
            ctx.request_repaint();
        }

        self.handle_keyboard_input(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let frame = egui::Frame::new().fill(Color32::from_rgb(15, 17, 23));
        frame.show(ui, |ui| {
            let available_size = ui.available_size();

            let font_id = FontId::monospace(self.font_size);
            let sample =
                ui.painter()
                    .layout_no_wrap("M".to_string(), font_id.clone(), Color32::WHITE);
            self.char_width = sample.size().x.max(4.0);
            self.char_height = sample.size().y.max(8.0);

            let margin = 8.0;
            let term_area_width = (available_size.x - (margin * 2.0)).max(100.0);
            let term_area_height = (available_size.y - (margin * 2.0)).max(100.0);

            let cols = (term_area_width / self.char_width).floor() as u16;
            let rows = (term_area_height / self.char_height).floor() as u16;
            let new_size = TerminalSize::new(cols, rows);

            if new_size != self.last_size {
                let _ = self.surface.resize(new_size);
                self.last_size = new_size;
            }

            let painter = ui.painter();
            let origin = Pos2::new(margin, margin);

            let current_rows = self.surface.rows().min(rows as usize);
            let current_cols = self.surface.cols().min(cols as usize);

            for r in 0..current_rows {
                let y = origin.y + (r as f32 * self.char_height);
                for c in 0..current_cols {
                    let cell = self.surface.get_cell(r, c);
                    let x = origin.x + (c as f32 * self.char_width);

                    if let Some(bg) = cell.bg {
                        let cell_rect = Rect::from_min_size(
                            Pos2::new(x, y),
                            Vec2::new(self.char_width, self.char_height),
                        );
                        painter.rect_filled(cell_rect, 0.0, Color32::from_rgb(bg.r, bg.g, bg.b));
                    }

                    if cell.c != ' ' && cell.c != '\0' {
                        let fg_color = if let Some(fg) = cell.fg {
                            Color32::from_rgb(fg.r, fg.g, fg.b)
                        } else {
                            Color32::from_rgb(220, 224, 232)
                        };

                        painter.text(
                            Pos2::new(x, y),
                            egui::Align2::LEFT_TOP,
                            cell.c.to_string(),
                            font_id.clone(),
                            fg_color,
                        );
                    }
                }
            }

            let (cursor_row, cursor_col) = self.surface.cursor_pos();
            if cursor_row < current_rows && cursor_col < current_cols {
                let cur_x = origin.x + (cursor_col as f32 * self.char_width);
                let cur_y = origin.y + (cursor_row as f32 * self.char_height);
                let cursor_rect = Rect::from_min_size(
                    Pos2::new(cur_x, cur_y),
                    Vec2::new(self.char_width, self.char_height),
                );

                painter.rect_filled(
                    cursor_rect,
                    1.0,
                    Color32::from_rgba_unmultiplied(230, 235, 245, 160),
                );
            }
        });
    }
}
