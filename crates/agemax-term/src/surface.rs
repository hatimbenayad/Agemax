use crate::pty::{PtyConfig, PtyError, PtyInstance};
use crate::vt::{RenderCell, TerminalGrid, TerminalGridConfig};
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
        let mut dirty = false;
        let mut grid = self.grid.write();
        while let Ok(chunk) = self.rx.try_recv() {
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

    pub fn cursor_pos(&self) -> (usize, usize) {
        self.grid.read().cursor_pos()
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
}
