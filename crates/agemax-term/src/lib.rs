//! Terminal abstraction, ConPTY integration, and VT buffer coalescing for Age Max.

pub mod coalescer;
pub mod job_object;
pub mod pty;
pub mod surface;
pub mod vt;

pub use coalescer::PtyCoalescer;
pub use job_object::JobObject;
pub use pty::{detect_default_shell, PtyConfig, PtyError, PtyInstance};
pub use surface::TerminalSurface;
pub use vt::{RenderCell, TerminalGrid, TerminalGridConfig};
