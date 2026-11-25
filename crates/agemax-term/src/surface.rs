use crate::pty::{PtyConfig, PtyError, PtyInstance};
use crate::vt::{GridPoint, RenderCell, TerminalGrid, TerminalGridConfig};
use agemax_core::model::{SurfaceId, TerminalSize};
use crossbeam_channel::Receiver;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct TerminalSurface {
    pub id: SurfaceId,
    pty: PtyInstance,
    rx: Receiver<Vec<u8>>,
    grid: Arc<RwLock<TerminalGrid>>,
    size: TerminalSize,
}

impl TerminalSurface {
    pub fn new(size: TerminalSize, mut config: PtyConfig) -> Result<Self, PtyError> {
        config.cols = size.cols;
        config.rows = size.rows;

        let (pty, rx) = PtyInstance::spawn(config)?;
        let grid = Arc::new(RwLock::new(TerminalGrid::new(TerminalGridConfig {
            cols: size.cols,
            rows: size.rows,
        })));

        Ok(Self {
            id: SurfaceId::new(),
            pty,
            rx,
            grid,
            size,
        })
    }

    /// Process any pending output from the PTY reader coalescer.
    /// Returns true if new data arrived and the screen should be repainted.
    pub fn process_output(&mut self) -> bool {
        self.process_output_with(|_| {})
    }

    /// Process pending output while providing access to each raw chunk.
    pub fn process_output_with<F: FnMut(&[u8])>(&mut self, mut on_chunk: F) -> bool {
        let mut dirty = false;
        let mut grid = self.grid.write();
        while let Ok(chunk) = self.rx.try_recv() {
            on_chunk(&chunk);
            grid.advance_bytes(&chunk);
            dirty = true;
        }
        dirty
    }

    pub fn resize(&mut self, size: TerminalSize) -> Result<(), PtyError> {
        if self.size == size {
            return Ok(());
        }
        self.size = size;
        self.grid.write().resize(size.cols, size.rows);
        self.pty.resize(size.cols, size.rows)
    }

    pub fn send_input(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.pty.write_all(bytes)
    }

    pub fn get_cell(&self, row: usize, col: usize) -> RenderCell {
        self.grid.read().get_cell(row, col)
    }

    pub fn get_cell_for_theme(
        &self,
        row: usize,
        col: usize,
        default_bg: (u8, u8, u8),
        default_fg: (u8, u8, u8),
    ) -> RenderCell {
        self.grid
            .read()
            .get_cell_for_theme(row, col, default_bg, default_fg)
    }

    pub fn cursor_pos(&self) -> (usize, usize) {
        self.grid.read().cursor_pos()
    }

    pub fn display_offset(&self) -> usize {
        self.grid.read().display_offset()
    }

    pub fn history_size(&self) -> usize {
        self.grid.read().history_size()
    }

    pub fn scroll_delta(&mut self, delta: i32) {
        self.grid.write().scroll_delta(delta);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.grid.write().scroll_to_bottom();
    }

    pub fn clear_screen(&mut self) {
        self.grid.write().clear_screen();
    }

    pub fn viewport_to_grid_line(&self, viewport_row: usize) -> i32 {
        self.grid.read().viewport_to_grid_line(viewport_row)
    }

    pub fn extract_selection_text(&self, start: GridPoint, end: GridPoint) -> String {
        self.grid.read().extract_selection_text(start, end)
    }

    pub fn url_at(&self, viewport_row: usize, col: usize) -> Option<String> {
        self.grid.read().url_at(viewport_row, col)
    }

    pub fn has_exited(&self) -> bool {
        self.pty.has_exited()
    }

    pub fn cols(&self) -> usize {
        self.size.cols as usize
    }

    pub fn rows(&self) -> usize {
        self.size.rows as usize
    }

    pub fn child_pid(&self) -> Option<u32> {
        self.pty.child_pid()
    }

    pub fn grid_arc(&self) -> Arc<RwLock<TerminalGrid>> {
        self.grid.clone()
    }

    /// Bottom-most non-empty visible terminal line, clamped for compact card previews.
    pub fn last_non_empty_line(&self) -> Option<String> {
        self.grid.read().last_non_empty_line()
    }
}
