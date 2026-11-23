# Unit 1: Single Window, Native ConPTY Surface & Input Loop

## 1. Scope
Deliver the core terminal execution engine: a single native window running an interactive shell session via Windows ConPTY, rendered through `alacritty_terminal` and `egui`, with full keyboard, mouse, and clipboard support.

## 2. Requirements Traceability
- **REQ-001**: ConPTY initialization via `portable-pty`.
- **REQ-002**: Automatic shell detection (PowerShell 7 -> Windows PowerShell -> cmd).
- **REQ-003**: Full VT escape sequence emulation (colors, alternate screen, cursor).
- **REQ-004**: Mouse event reporting (SGR mode 1006).
- **REQ-005**: OSC 7/8/52 handling.
- **REQ-006**: Terminal resize and text reflow.
- **REQ-007**: Scrollback, selection, and clipboard copy/paste.
- **REQ-008**: Dedicated PTY reader background thread.
- **REQ-009**: ~4 ms PTY output batching and coalescing.
- **REQ-010**: Windows Job Object process tree containment.
- **REQ-058**: Cold start < 300 ms.
- **REQ-059**: Keystroke-to-glyph latency < 16 ms.

## 3. Interfaces & Data Structures
```rust
pub struct TerminalSurface {
    pub id: SurfaceId,
    pub term: Arc<RwLock<Term<AlacrittyEventListener>>>,
    pub pty_writer: Box<dyn std::io::Write + Send>,
    pub job_object: Arc<JobObjectHandle>,
    pub size: TerminalSize,
}

pub trait PtySystemBackend: Send + Sync {
    fn spawn(&self, config: &PtyConfig) -> Result<Box<dyn PtyInstance>, TermError>;
}
```

## 4. Acceptance Criteria
1. Launching `agemax` opens a high-DPI native window within 300 ms.
2. A PowerShell session is immediately interactive. Keystroke-to-glyph latency is < 16 ms.
3. Complex interactive CLI programs (e.g., `vim`, `fzf`, `htop`, `git log`) render with correct ANSI styling, alternate screen transitions, and cursor positioning.
4. Window resizing reflows terminal text and synchronizes rows/columns with ConPTY.
5. Mouse wheel scrolls history; text selection and copy (`Ctrl+Shift+C`) / paste (`Ctrl+Shift+V`) work seamlessly.
6. Closing the window or terminating the surface terminates all child processes in the Job Object without leaving orphan processes.

## 5. Test Plan
- Automated unit test verifying VT parsing and cursor movements in `agemax-term`.
- PTY throughput benchmark (`vtebench` style) verifying > 10 MB/s VT parsing.
- Job Object unit test verifying child process tree auto-termination upon handle drop.
- Manual verification on Windows 10/11: font DPI scaling, IME input, clipboard roundtrip.
