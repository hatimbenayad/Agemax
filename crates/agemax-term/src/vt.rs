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
        let fg = convert_color(cell.fg);
        let bg = convert_color(cell.bg);
        let flags = cell.flags;

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

fn convert_color(c: Color) -> Option<CellRgb> {
    match c {
        Color::Spec(rgb) => Some(CellRgb {
            r: rgb.r,
            g: rgb.g,
            b: rgb.b,
        }),
        Color::Named(named) => {
            let (r, g, b) = match named {
                NamedColor::Black => (0, 0, 0),
                NamedColor::Red => (205, 49, 49),
                NamedColor::Green => (13, 188, 121),
                NamedColor::Yellow => (229, 229, 16),
                NamedColor::Blue => (36, 114, 200),
                NamedColor::Magenta => (188, 63, 188),
                NamedColor::Cyan => (17, 168, 205),
                NamedColor::White => (229, 229, 229),
                NamedColor::BrightBlack => (102, 102, 102),
                NamedColor::BrightRed => (241, 76, 76),
                NamedColor::BrightGreen => (35, 209, 139),
                NamedColor::BrightYellow => (245, 245, 67),
                NamedColor::BrightBlue => (59, 142, 234),
                NamedColor::BrightMagenta => (214, 112, 214),
                NamedColor::BrightCyan => (41, 184, 219),
                NamedColor::BrightWhite => (255, 255, 255),
                _ => return None,
            };
            Some(CellRgb { r, g, b })
        }
        Color::Indexed(idx) => Some(CellRgb {
            r: idx,
            g: idx,
            b: idx,
        }),
    }
}
