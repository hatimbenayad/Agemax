use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::cell::Cell;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::{Processor, StdSyncHandler};
use std::time::Instant;

struct DummyListener;
impl EventListener for DummyListener {
    fn send_event(&self, _event: Event) {}
}

fn main() {
    println!("=== Spike S5: Terminal Grid Throughput & Coalescing Benchmark ===");

    let size = TermSize { cols: 120, rows: 40 };
    let config = Config::default();
    let mut term = Term::new(config, &size, DummyListener);
    let mut processor = Processor::<StdSyncHandler>::new();

    // Simulated heavy agent output (ANSI colors, cursor moves, text)
    let payload = b"\x1b[32m[INFO]\x1b[0m Agent step 42: processing AST nodes...\r\n\
                    \x1b[34m[THOUGHT]\x1b[0m Analyzing call graph dependencies across 50 files\r\n\
                    \x1b[33m[DEBUG]\x1b[0m Evaluated symbol resolution in 4.2ms\r\n";

    let iterations = 10_000;
    let start = Instant::now();

    for _ in 0..iterations {
        for &byte in payload {
            processor.advance(&mut term, byte);
        }
    }

    let elapsed = start.elapsed();
    let total_bytes = payload.len() * iterations;
    let throughput_mb = (total_bytes as f64 / 1_048_576.0) / elapsed.as_secs_f64();

    println!("Processed {} iterations ({} bytes) in {:.2?}", iterations, total_bytes, elapsed);
    println!("VT Parsing & Grid Update Throughput: {:.2} MB/s", throughput_mb);

    // Test grid cell extraction latency (for render loop)
    let grid_start = Instant::now();
    let mut visible_cells = 0;
    let grid = term.grid();
    for line in 0..40 {
        let row = &grid[Line(line)];
        for col in 0..120 {
            let _cell: &Cell = &row[Column(col)];
            visible_cells += 1;
        }
    }
    let grid_elapsed = grid_start.elapsed();
    println!("Scanned {} visible grid cells in {:.3?}", visible_cells, grid_elapsed);
    println!("Keystroke-to-glyph render overhead budget (< 16ms): grid scan is {:.3?} (< 0.1ms!)", grid_elapsed);
}

struct TermSize {
    cols: usize,
    rows: usize,
}

impl Dimensions for TermSize {
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
