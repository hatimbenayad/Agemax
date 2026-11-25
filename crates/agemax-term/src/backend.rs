//! Execution backends for terminal panes (`LocalConPtyBackend`, `WslPtyBackend`, `SshPtyBackend`).

use crate::pty::{resolve_shell_program, PtyConfig};
use agemax_core::model::TerminalSize;
use agemax_core::workspace::WorkspaceBackend;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Abstraction over how a terminal session starts its shell environment.
pub trait PtySystemBackend: Send + Sync {
    fn kind_label(&self) -> &'static str;

    fn build_pty_config(
        &self,
        local_cwd: &Path,
        shell_pref: Option<&str>,
        size: TerminalSize,
        extra_env: HashMap<String, String>,
    ) -> PtyConfig;
}

/// Default Windows native ConPTY backend.
#[derive(Debug, Clone, Default)]
pub struct LocalConPtyBackend;

impl PtySystemBackend for LocalConPtyBackend {
    fn kind_label(&self) -> &'static str {
        "Local"
    }

    fn build_pty_config(
        &self,
        local_cwd: &Path,
        shell_pref: Option<&str>,
        size: TerminalSize,
        extra_env: HashMap<String, String>,
    ) -> PtyConfig {
        PtyConfig {
            program: resolve_shell_program(shell_pref),
            args: Vec::new(),
            env: extra_env,
            cwd: Some(local_cwd.to_path_buf()),
            cols: size.cols,
            rows: size.rows,
        }
    }
}

/// Windows Subsystem for Linux (`wsl.exe`) backend.
#[derive(Debug, Clone, Default)]
pub struct WslPtyBackend {
    pub distro: Option<String>,
    pub linux_cwd: Option<String>,
}

impl PtySystemBackend for WslPtyBackend {
    fn kind_label(&self) -> &'static str {
        "WSL"
    }

    fn build_pty_config(
        &self,
        local_cwd: &Path,
        _shell_pref: Option<&str>,
        size: TerminalSize,
        extra_env: HashMap<String, String>,
    ) -> PtyConfig {
        let program = resolve_shell_program(Some("wsl"));
        let mut args = Vec::new();
        if let Some(d) = self
            .distro
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            args.push("-d".to_string());
            args.push(d.to_string());
        }
        if let Some(dir) = self
            .linux_cwd
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            args.push("--cd".to_string());
            args.push(dir.to_string());
        }

        PtyConfig {
            program,
            args,
            env: extra_env,
            cwd: Some(local_cwd.to_path_buf()),
            cols: size.cols,
            rows: size.rows,
        }
    }
}

/// Remote SSH workspace backend delegating auth to system OpenSSH (`ssh.exe` / user keys / ssh-agent).
#[derive(Debug, Clone)]
pub struct SshPtyBackend {
    pub target: String,
    pub port: Option<u16>,
    pub remote_dir: Option<String>,
}

impl PtySystemBackend for SshPtyBackend {
    fn kind_label(&self) -> &'static str {
        "SSH"
    }

    fn build_pty_config(
        &self,
        local_cwd: &Path,
        _shell_pref: Option<&str>,
        size: TerminalSize,
        extra_env: HashMap<String, String>,
    ) -> PtyConfig {
        let mut args = vec!["-t".to_string()];
        if let Some(p) = self.port {
            args.push("-p".to_string());
            args.push(p.to_string());
        }
        args.push(self.target.trim().to_string());

        if let Some(dir) = self
            .remote_dir
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            // Single-quote remote path safely for POSIX remote shells.
            let quoted = dir.replace('\'', "'\\''");
            args.push(format!("cd '{quoted}' && exec ${{SHELL:-/bin/sh}} -l"));
        }

        PtyConfig {
            program: resolve_ssh_program(),
            args,
            env: extra_env,
            cwd: Some(local_cwd.to_path_buf()),
            cols: size.cols,
            rows: size.rows,
        }
    }
}

pub fn backend_from_workspace(backend: &WorkspaceBackend) -> Box<dyn PtySystemBackend> {
    match backend {
        WorkspaceBackend::Local => Box::new(LocalConPtyBackend),
        WorkspaceBackend::Wsl { distro, linux_cwd } => Box::new(WslPtyBackend {
            distro: distro.clone(),
            linux_cwd: linux_cwd.clone(),
        }),
        WorkspaceBackend::Ssh {
            target,
            port,
            remote_dir,
        } => Box::new(SshPtyBackend {
            target: target.clone(),
            port: *port,
            remote_dir: remote_dir.clone(),
        }),
    }
}

fn resolve_ssh_program() -> PathBuf {
    #[cfg(windows)]
    {
        let sys_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
        let openssh = PathBuf::from(&sys_root)
            .join("System32")
            .join("OpenSSH")
            .join("ssh.exe");
        if openssh.exists() {
            return openssh;
        }
    }
    PathBuf::from("ssh")
}

/// Parse stdout bytes from `wsl.exe --list --quiet` (handles UTF-16LE and UTF-8).
pub fn parse_wsl_list_output(bytes: &[u8]) -> Vec<String> {
    let (pairs, _) = bytes.as_chunks::<2>();
    let text = if !pairs.is_empty() && pairs.iter().any(|c| c[1] == 0) {
        let units: Vec<u16> = pairs.iter().map(|c| u16::from_le_bytes(*c)).collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    };

    text.lines()
        .map(|l| l.trim().trim_start_matches('\u{feff}'))
        .filter(|l| !l.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// Query installed WSL distributions via `wsl.exe --list --quiet`.
pub fn detect_wsl_distros() -> Vec<String> {
    #[cfg(windows)]
    {
        let Ok(out) = std::process::Command::new("wsl.exe")
            .args(["--list", "--quiet"])
            .output()
        else {
            return Vec::new();
        };
        if !out.status.success() {
            return Vec::new();
        }
        parse_wsl_list_output(&out.stdout)
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}
