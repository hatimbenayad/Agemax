use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::{
    ClearMode, Color, Handler, NamedColor, Processor, StdSyncHandler,
};

/// Logical grid coordinate anchored to scrollback history (`Line(0)` = live top row; negative = scrollback).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GridPoint {
    pub line: i32,
    pub col: usize,
}

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

    pub fn display_offset(&self) -> usize {
        self.term.grid().display_offset()
    }

    pub fn history_size(&self) -> usize {
        self.term.grid().history_size()
    }

    pub fn scroll_delta(&mut self, delta: i32) {
        self.term.scroll_display(Scroll::Delta(delta));
    }

    pub fn scroll_to_bottom(&mut self) {
        self.term.scroll_display(Scroll::Bottom);
    }

    pub fn clear_screen(&mut self) {
        self.term.clear_screen(ClearMode::All);
        self.term.scroll_display(Scroll::Bottom);
    }

    /// Translate a viewport row (`0..rows`) into an absolute `alacritty_terminal` grid line index.
    pub fn viewport_to_grid_line(&self, viewport_row: usize) -> i32 {
        viewport_row as i32 - self.display_offset() as i32
    }

    pub fn get_cell(&self, row: usize, col: usize) -> RenderCell {
        self.get_cell_for_theme(row, col, (16, 18, 26), (220, 224, 235))
    }

    /// Fetch a cell adjusted for scrollback offset and contrast-protected against the active theme colors.
    pub fn get_cell_for_theme(
        &self,
        row: usize,
        col: usize,
        default_bg: (u8, u8, u8),
        default_fg: (u8, u8, u8),
    ) -> RenderCell {
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
        let line_idx = row as i32 - grid.display_offset() as i32;
        let cell: &Cell = &grid[Line(line_idx)][Column(col)];
        let c = cell.c;
        let mut fg = convert_color(cell.fg);
        let mut bg = convert_color(cell.bg);
        let flags = cell.flags;

        if flags.contains(Flags::INVERSE) {
            std::mem::swap(&mut fg, &mut bg);
            if fg.is_none() {
                fg = Some(CellRgb {
                    r: default_bg.0,
                    g: default_bg.1,
                    b: default_bg.2,
                });
            }
            if bg.is_none() {
                bg = Some(CellRgb {
                    r: default_fg.0,
                    g: default_fg.1,
                    b: default_fg.2,
                });
            }
        }

        // Contrast guarantee: ensure foreground is always clearly readable against background
        if let Some(ref mut fg_rgb) = fg {
            let fg_lum = luminance(fg_rgb.r, fg_rgb.g, fg_rgb.b);
            let bg_lum = if let Some(ref bg_rgb) = bg {
                luminance(bg_rgb.r, bg_rgb.g, bg_rgb.b)
            } else {
                luminance(default_bg.0, default_bg.1, default_bg.2)
            };

            if fg_lum.abs_diff(bg_lum) < 45 {
                if bg_lum < 128 {
                    fg_rgb.r = 220;
                    fg_rgb.g = 224;
                    fg_rgb.b = 235;
                } else {
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
        self.render_grid_line_text(row as i32)
    }

    /// Render the line currently shown at `viewport_row`, accounting for active scrollback offset.
    pub fn render_viewport_line_text(&self, viewport_row: usize) -> String {
        if viewport_row >= self.rows {
            return String::new();
        }
        let line_idx = self.viewport_to_grid_line(viewport_row);
        self.render_grid_line_text(line_idx)
    }

    fn render_grid_line_text(&self, line_idx: i32) -> String {
        let min_line = -(self.history_size() as i32);
        let max_line = self.rows as i32 - 1;
        if line_idx < min_line || line_idx > max_line {
            return String::new();
        }
        let grid = self.term.grid();
        let row_line = &grid[Line(line_idx)];
        let mut s = String::with_capacity(self.cols);
        for col in 0..self.cols {
            let ch = row_line[Column(col)].c;
            s.push(if ch == '\0' { ' ' } else { ch });
        }
        s
    }

    /// Extract clean, right-trimmed multi-line text between two logical grid coordinates.
    pub fn extract_selection_text(&self, start: GridPoint, end: GridPoint) -> String {
        let (a, b) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        if self.cols == 0 || self.rows == 0 {
            return String::new();
        }
        let min_line = -(self.history_size() as i32);
        let max_line = self.rows as i32 - 1;
        let line_start = a.line.clamp(min_line, max_line);
        let line_end = b.line.clamp(min_line, max_line);
        if line_start > line_end {
            return String::new();
        }

        let mut lines_out = Vec::new();
        for line in line_start..=line_end {
            let row_str = self.render_grid_line_text(line);
            let chars: Vec<char> = row_str.chars().collect();
            let c_start = if line == a.line {
                a.col.min(chars.len().saturating_sub(1))
            } else {
                0
            };
            let c_end = if line == b.line {
                b.col.min(chars.len().saturating_sub(1))
            } else {
                chars.len().saturating_sub(1)
            };
            if c_start <= c_end && c_start < chars.len() {
                let slice: String = chars[c_start..=c_end].iter().collect();
                lines_out.push(slice.trim_end().to_string());
            } else {
                lines_out.push(String::new());
            }
        }
        lines_out.join("\n")
    }

    /// Resolve any OSC 8 hyperlink or plain HTTP/HTTPS URL at a given viewport cell.
    pub fn url_at(&self, viewport_row: usize, col: usize) -> Option<String> {
        if viewport_row >= self.rows || col >= self.cols {
            return None;
        }
        let line_idx = self.viewport_to_grid_line(viewport_row);
        let grid = self.term.grid();
        if let Some(link) = grid[Line(line_idx)][Column(col)].hyperlink() {
            return Some(link.uri().to_string());
        }

        let line = self.render_grid_line_text(line_idx);
        find_http_url_at_col(&line, col)
    }

    /// Extract the bottom-most non-empty trimmed line from the visible viewport (for Mission Control cards).
    pub fn last_non_empty_line(&self) -> Option<String> {
        let mut r = self.rows;
        while r > 0 {
            r -= 1;
            let text = self.render_viewport_line_text(r);
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                let mut out: String = trimmed.chars().take(120).collect();
                if trimmed.chars().count() > 120 {
                    out.push('…');
                }
                return Some(out);
            }
        }
        None
    }

    pub fn term(&self) -> &Term<NullEventListener> {
        &self.term
    }

    pub fn term_mut(&mut self) -> &mut Term<NullEventListener> {
        &mut self.term
    }
}

fn luminance(r: u8, g: u8, b: u8) -> u32 {
    (r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000
}

fn find_http_url_at_col(line: &str, col: usize) -> Option<String> {
    let mut search_from = 0;
    while let Some(rel) = line[search_from..]
        .find("https://")
        .or_else(|| line[search_from..].find("http://"))
    {
        let abs_start = search_from + rel;
        let tail = &line[abs_start..];
        let raw_end = tail
            .find(|c: char| c.is_whitespace() || matches!(c, '<' | '>' | '"' | '\''))
            .map(|len| abs_start + len)
            .unwrap_or(line.len());

        let clean_slice =
            line[abs_start..raw_end].trim_end_matches(['.', ',', ';', ':', ')', ']', '!']);
        let char_start = line[..abs_start].chars().count();
        let char_len = clean_slice.chars().count();

        if col >= char_start && col < char_start + char_len && !clean_slice.is_empty() {
            return Some(clean_slice.to_string());
        }
        search_from = raw_end.max(abs_start + 7);
        if search_from >= line.len() {
            break;
        }
    }
    None
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
