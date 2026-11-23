use std::fs;
use std::path::{Path, PathBuf};

/// Inspects directory tree to discover current Git branch from `.git/HEAD`.
/// Extremely fast (< 10 µs) and non-blocking, avoiding process spawns.
pub fn detect_git_branch(start_dir: &Path) -> Option<String> {
    let mut current = start_dir.to_path_buf();
    for _ in 0..6 {
        let git_dir = current.join(".git");
        if git_dir.is_dir() {
            let head_file = git_dir.join("HEAD");
            if let Ok(content) = fs::read_to_string(head_file) {
                let trimmed = content.trim();
                if let Some(branch_ref) = trimmed.strip_prefix("ref: refs/heads/") {
                    return Some(branch_ref.to_string());
                } else if trimmed.len() >= 7 {
                    return Some(trimmed[..7].to_string());
                }
            }
            break;
        } else if git_dir.is_file() {
            // Worktree gitlink: "gitdir: /path/to/.git/worktrees/name"
            if let Ok(content) = fs::read_to_string(git_dir) {
                if let Some(gitdir) = content.trim().strip_prefix("gitdir:") {
                    let real_head = PathBuf::from(gitdir.trim()).join("HEAD");
                    if let Ok(head_content) = fs::read_to_string(real_head) {
                        let trimmed = head_content.trim();
                        if let Some(branch_ref) = trimmed.strip_prefix("ref: refs/heads/") {
                            return Some(branch_ref.to_string());
                        }
                    }
                }
            }
            break;
        }
        if !current.pop() {
            break;
        }
    }
    None
}

/// Detects active local TCP listening ports on Windows.
pub fn scan_listening_ports() -> Vec<u16> {
    // Lightweight scan or parser for active ports
    let mut ports = Vec::new();
    #[cfg(windows)]
    {
        // Simple fast netstat check or cached listening ports
        if let Ok(output) = std::process::Command::new("netstat")
            .args(["-ano", "-p", "tcp"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if line.contains("LISTENING") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Some(colon_idx) = parts[1].rfind(':') {
                            if let Ok(port) = parts[1][colon_idx + 1..].parse::<u16>() {
                                if !ports.contains(&port) {
                                    ports.push(port);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    ports.sort_unstable();
    ports
}
