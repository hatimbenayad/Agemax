//! Terminal abstraction, ConPTY integration, and VT buffer coalescing for Age Max.

pub mod backend;
pub mod coalescer;
pub mod job_object;
pub mod metadata;
pub mod pty;
pub mod surface;
pub mod vt;

pub use backend::{
    backend_from_workspace, detect_wsl_distros, LocalConPtyBackend, PtySystemBackend,
    SshPtyBackend, WslPtyBackend,
};
pub use coalescer::PtyCoalescer;
pub use job_object::JobObject;
pub use metadata::{detect_git_branch, scan_listening_ports};
pub use pty::{detect_default_shell, PtyConfig, PtyError, PtyInstance};
pub use surface::TerminalSurface;
pub use vt::{RenderCell, TerminalGrid, TerminalGridConfig};
