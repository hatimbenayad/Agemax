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

/// Aggregated line churn statistics for a workspace working tree (`REQ-087`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GitDiffStat {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

impl GitDiffStat {
    pub fn has_changes(&self) -> bool {
        self.insertions > 0 || self.deletions > 0 || self.files_changed > 0
    }
}

/// Parses output of `git diff --shortstat` (e.g., `" 3 files changed, 42 insertions(+), 7 deletions(-)"`).
pub fn parse_git_shortstat(output: &str) -> GitDiffStat {
    let mut stat = GitDiffStat::default();
    for segment in output.split(',') {
        let s = segment.trim();
        let num = s
            .split_whitespace()
            .next()
            .and_then(|tok| tok.parse::<usize>().ok());
        if let Some(val) = num {
            if s.contains("file") {
                stat.files_changed = val;
            } else if s.contains("insertion") {
                stat.insertions = val;
            } else if s.contains("deletion") {
                stat.deletions = val;
            }
        }
    }
    stat
}

/// Runs `git diff --shortstat` on `repo_dir`. MUST only be invoked from throttled background worker threads.
pub fn detect_git_diff_stats(repo_dir: &Path) -> Option<GitDiffStat> {
    let mut cmd = std::process::Command::new("git");
    cmd.args(["-C"]).arg(repo_dir).args(["diff", "--shortstat"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Some(parse_git_shortstat(&stdout))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_git_shortstat_variants() {
        assert_eq!(
            parse_git_shortstat(" 3 files changed, 42 insertions(+), 7 deletions(-)\n"),
            GitDiffStat {
                files_changed: 3,
                insertions: 42,
                deletions: 7,
            }
        );
        assert_eq!(
            parse_git_shortstat(" 1 file changed, 15 insertions(+)"),
            GitDiffStat {
                files_changed: 1,
                insertions: 15,
                deletions: 0,
            }
        );
        assert_eq!(parse_git_shortstat(""), GitDiffStat::default());
    }
}
