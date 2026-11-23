use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::{Color, NamedColor, Processor, StdSyncHandler};

#[derive(Clone, Copy, Debug)]
pub struct TerminalGridConfig {
    pub cols: u16,
    pub rows: u16,
}

impl Default for TerminalGridConfig {
    fn default() -> Self {
        Self { cols: 80, rows: 24 }
    }
}

pub struct TerminalSizeDims {
    pub cols: usize,
    pub rows: usize,
}

impl Dimensions for TerminalSizeDims {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

#[derive(Default)]
pub struct NullEventListener;
impl EventListener for NullEventListener {
    fn send_event(&self, _event: Event) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone)]
pub struct RenderCell {
    pub c: char,
    pub fg: Option<CellRgb>,
    pub bg: Option<CellRgb>,
    pub bold: bool,
    pub underline: bool,
    pub italic: bool,
}

pub struct TerminalGrid {
    term: Term<NullEventListener>,
    processor: Processor<StdSyncHandler>,
    cols: usize,
    rows: usize,
}

impl TerminalGrid {
    pub fn new(config: TerminalGridConfig) -> Self {
        let cols = config.cols.max(1) as usize;
        let rows = config.rows.max(1) as usize;
        let dims = TerminalSizeDims { cols, rows };
        let term_config = Config::default();
        let term = Term::new(term_config, &dims, NullEventListener);
        let processor = Processor::<StdSyncHandler>::new();

        Self {
            term,
            processor,
            cols,
            rows,
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        let cols = cols.max(1) as usize;
        let rows = rows.max(1) as usize;
        if self.cols == cols && self.rows == rows {
            return;
        }
        self.cols = cols;
        self.rows = rows;
        let dims = TerminalSizeDims { cols, rows };
        self.term.resize(dims);
    }

    pub fn advance_bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.processor.advance(&mut self.term, byte);
        }
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cursor_pos(&self) -> (usize, usize) {
        let point = self.term.grid().cursor.point;
        (point.line.0 as usize, point.column.0)
    }

    pub fn get_cell(&self, row: usize, col: usize) -> RenderCell {
        if row >= self.rows || col >= self.cols {
            return RenderCell {
                c: ' ',
                fg: None,
                bg: None,
                bold: false,
                underline: false,
                italic: false,
            };
        }

        let grid = self.term.grid();
        let cell: &Cell = &grid[Line(row as i32)][Column(col)];
        let c = cell.c;
        let mut fg = convert_color(cell.fg);
        let mut bg = convert_color(cell.bg);
        let flags = cell.flags;

        if flags.contains(Flags::INVERSE) {
            std::mem::swap(&mut fg, &mut bg);
            if fg.is_none() {
                fg = Some(CellRgb {
                    r: 16,
                    g: 18,
                    b: 26,
                });
            }
            if bg.is_none() {
                bg = Some(CellRgb {
                    r: 220,
                    g: 224,
                    b: 235,
                });
            }
        }

        // Contrast guarantee: ensure foreground is always clearly readable against background
        if let Some(ref mut fg_rgb) = fg {
            let fg_lum =
                (fg_rgb.r as u32 * 299 + fg_rgb.g as u32 * 587 + fg_rgb.b as u32 * 114) / 1000;
            let bg_lum = if let Some(ref bg_rgb) = bg {
                (bg_rgb.r as u32 * 299 + bg_rgb.g as u32 * 587 + bg_rgb.b as u32 * 114) / 1000
            } else {
                18 // default dark pane background
            };

            // If contrast difference is too low (< 45), boost or adjust foreground
            if fg_lum.abs_diff(bg_lum) < 45 {
                if bg_lum < 128 {
                    // Dark background: boost foreground to crisp readable light gray
                    fg_rgb.r = 220;
                    fg_rgb.g = 224;
                    fg_rgb.b = 235;
                } else {
                    // Light background: darken foreground to high-contrast dark slate
                    fg_rgb.r = 20;
                    fg_rgb.g = 22;
                    fg_rgb.b = 30;
                }
            }
        }

        RenderCell {
            c,
            fg,
            bg,
            bold: flags.contains(Flags::BOLD),
            underline: flags.contains(Flags::UNDERLINE),
            italic: flags.contains(Flags::ITALIC),
        }
    }

    pub fn render_line_text(&self, row: usize) -> String {
        if row >= self.rows {
            return String::new();
        }
        let grid = self.term.grid();
        let row_line = &grid[Line(row as i32)];
        let mut s = String::with_capacity(self.cols);
        for col in 0..self.cols {
            s.push(row_line[Column(col)].c);
        }
        s
    }

    pub fn term(&self) -> &Term<NullEventListener> {
        &self.term
    }

    pub fn term_mut(&mut self) -> &mut Term<NullEventListener> {
        &mut self.term
    }
}

pub fn indexed_to_rgb(idx: u8) -> (u8, u8, u8) {
    match idx {
        // Standard ANSI 16 colors (modern high-contrast dark theme palette)
        0 => (40, 44, 56),   // Black / Dark Slate (visible dark, never pure 0,0,0)
        1 => (235, 75, 75),  // Red
        2 => (50, 205, 120), // Green
        3 => (245, 190, 50), // Yellow
        4 => (75, 145, 245), // Blue
        5 => (200, 95, 230), // Magenta
        6 => (45, 210, 225), // Cyan
        7 => (220, 224, 235), // Light Gray / White
        8 => (95, 105, 125), // Bright Black / Slate Gray
        9 => (255, 100, 100), // Bright Red
        10 => (75, 230, 145), // Bright Green
        11 => (255, 215, 75), // Bright Yellow
        12 => (105, 175, 255), // Bright Blue
        13 => (225, 130, 255), // Bright Magenta
        14 => (75, 235, 245), // Bright Cyan
        15 => (255, 255, 255), // Bright White
        // 6x6x6 color cube: 16..=231
        16..=231 => {
            let offset = idx - 16;
            let r_step = offset / 36;
            let g_step = (offset % 36) / 6;
            let b_step = offset % 6;
            let to_val = |s: u8| if s == 0 { 0 } else { 55 + s * 40 };
            (to_val(r_step), to_val(g_step), to_val(b_step))
        }
        // Grayscale ramp: 232..=255
        232..=255 => {
            let gray = 8 + (idx - 232) * 10;
            (gray, gray, gray)
        }
    }
}

fn convert_color(c: Color) -> Option<CellRgb> {
    match c {
        Color::Spec(rgb) => Some(CellRgb {
            r: rgb.r,
            g: rgb.g,
            b: rgb.b,
        }),
        Color::Named(named) => {
            let (r, g, b) = match named {
                NamedColor::Black => (40, 44, 56),
                NamedColor::Red => (235, 75, 75),
                NamedColor::Green => (50, 205, 120),
                NamedColor::Yellow => (245, 190, 50),
                NamedColor::Blue => (75, 145, 245),
                NamedColor::Magenta => (200, 95, 230),
                NamedColor::Cyan => (45, 210, 225),
                NamedColor::White => (220, 224, 235),
                NamedColor::BrightBlack => (95, 105, 125),
                NamedColor::BrightRed => (255, 100, 100),
                NamedColor::BrightGreen => (75, 230, 145),
                NamedColor::BrightYellow => (255, 215, 75),
                NamedColor::BrightBlue => (105, 175, 255),
                NamedColor::BrightMagenta => (225, 130, 255),
                NamedColor::BrightCyan => (75, 235, 245),
                NamedColor::BrightWhite => (255, 255, 255),
                _ => return None,
            };
            Some(CellRgb { r, g, b })
        }
        Color::Indexed(idx) => {
            let (r, g, b) = indexed_to_rgb(idx);
            Some(CellRgb { r, g, b })
        }
    }
}
