use crate::coalescer::PtyCoalescer;
use crate::job_object::JobObject;
use crossbeam_channel::Receiver;
use portable_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PtyError {
    #[error("Failed to open PTY: {0}")]
    OpenFailed(String),
    #[error("Failed to spawn process in PTY: {0}")]
    SpawnFailed(String),
    #[error("Job Object error: {0}")]
    JobObject(#[from] crate::job_object::JobObjectError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

fn find_in_path(binary: &str) -> Option<PathBuf> {
    if let Some(paths) = std::env::var_os("PATH") {
        for p in std::env::split_paths(&paths) {
            let candidate = p.join(binary);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Detect default shell in order of preference:
/// 1. PowerShell 7 (`pwsh.exe`)
/// 2. Windows PowerShell (`powershell.exe`)
/// 3. Command Prompt (`cmd.exe`)
pub fn detect_default_shell() -> PathBuf {
    #[cfg(windows)]
    {
        // 1. Try pwsh
        if let Some(path) = find_in_path("pwsh.exe") {
            return path;
        }

        // 2. Try Windows PowerShell
        let sys_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
        let powershell_path = PathBuf::from(&sys_root)
            .join("System32")
            .join("WindowsPowerShell")
            .join("v1.0")
            .join("powershell.exe");
        if powershell_path.exists() {
            return powershell_path;
        }
        if let Some(path) = find_in_path("powershell.exe") {
            return path;
        }

        // 3. Fallback to cmd.exe
        let cmd_path = PathBuf::from(&sys_root).join("System32").join("cmd.exe");
        if cmd_path.exists() {
            return cmd_path;
        }
        PathBuf::from("cmd.exe")
    }

    #[cfg(not(windows))]
    {
        if let Ok(shell) = std::env::var("SHELL") {
            return PathBuf::from(shell);
        }
        PathBuf::from("/bin/sh")
    }
}

pub struct PtyConfig {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub cwd: Option<PathBuf>,
    pub cols: u16,
    pub rows: u16,
}

impl Default for PtyConfig {
    fn default() -> Self {
        Self {
            program: detect_default_shell(),
            args: Vec::new(),
            env: HashMap::new(),
            cwd: None,
            cols: 80,
            rows: 24,
        }
    }
}

pub struct PtyInstance {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    job_object: Arc<JobObject>,
    child_pid: Option<u32>,
    exit_signal: Arc<AtomicBool>,
}

impl PtyInstance {
    pub fn spawn(config: PtyConfig) -> Result<(Self, Receiver<Vec<u8>>), PtyError> {
        let pty_system = NativePtySystem::default();
        let pair = pty_system
            .openpty(PtySize {
                rows: config.rows.max(1),
                cols: config.cols.max(1),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::OpenFailed(e.to_string()))?;

        let mut cmd = CommandBuilder::new(&config.program);
        for arg in &config.args {
            cmd.arg(arg);
        }
        if let Some(cwd) = &config.cwd {
            cmd.cwd(cwd);
        }
        for (k, v) in &config.env {
            cmd.env(k, v);
        }

        // Child process environment defaults for clean terminal emulation & agent integration
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("AGY_CLI_OSC7501", "1");

        // Augment PATH with common agent CLI install locations (~/.gemini/bin, %LOCALAPPDATA%\agy\bin)
        let current_path = std::env::var("PATH").unwrap_or_default();
        let mut path_dirs = std::env::split_paths(&current_path).collect::<Vec<_>>();
        let mut path_updated = false;

        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            let gemini_bin = PathBuf::from(userprofile).join(".gemini").join("bin");
            if gemini_bin.exists() && !path_dirs.contains(&gemini_bin) {
                path_dirs.insert(0, gemini_bin);
                path_updated = true;
            }
        }
        if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
            let agy_bin = PathBuf::from(local_appdata).join("agy").join("bin");
            if agy_bin.exists() && !path_dirs.contains(&agy_bin) {
                path_dirs.insert(0, agy_bin);
                path_updated = true;
            }
        }

        if path_updated {
            if let Ok(new_path) = std::env::join_paths(&path_dirs) {
                cmd.env("PATH", new_path);
            }
        }

        // Create Windows Job Object to contain child process tree
        let job_object = Arc::new(JobObject::create()?);

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| PtyError::SpawnFailed(e.to_string()))?;
        drop(pair.slave); // Master holds the other end

        let child_pid = child.process_id();
        if let Some(pid) = child_pid {
            let _ = job_object.assign_pid(pid);
        }

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| PtyError::OpenFailed(format!("Failed to clone reader: {}", e)))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::OpenFailed(format!("Failed to take writer: {}", e)))?;

        let (mut coalescer, rx) = PtyCoalescer::new(Duration::from_millis(4));
        let exit_signal = Arc::new(AtomicBool::new(false));
        let exit_clone = exit_signal.clone();

        // Background reader thread: blocks on PTY stream and feeds coalescer
        thread::Builder::new()
            .name("agemax-pty-reader".into())
            .spawn(move || {
                let mut buf = [0u8; 8192];
                while !exit_clone.load(Ordering::Relaxed) {
                    match reader.read(&mut buf) {
                        Ok(0) => break, // EOF
                        Ok(n) => {
                            coalescer.push_bytes(&buf[..n]);
                        }
                        Err(_) => break,
                    }
                }
                coalescer.flush();
            })
            .map_err(|e| PtyError::SpawnFailed(format!("Failed to spawn reader thread: {}", e)))?;

        let instance = Self {
            master: pair.master,
            writer,
            job_object,
            child_pid,
            exit_signal,
        };

        Ok((instance, rx))
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), PtyError> {
        self.master
            .resize(PtySize {
                rows: rows.max(1),
                cols: cols.max(1),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::OpenFailed(format!("Resize failed: {}", e)))
    }

    pub fn write_all(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.writer.write_all(bytes)?;
        self.writer.flush()
    }

    pub fn child_pid(&self) -> Option<u32> {
        self.child_pid
    }

    pub fn job_object(&self) -> &Arc<JobObject> {
        &self.job_object
    }

    pub fn kill(&self) {
        self.exit_signal.store(true, Ordering::Relaxed);
        // Dropping or closing JobObject terminates process tree automatically on Windows
    }
}

impl Drop for PtyInstance {
    fn drop(&mut self) {
        self.kill();
    }
}
