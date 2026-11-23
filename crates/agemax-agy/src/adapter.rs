use crate::profile::ProfileConfig;
use crate::session_capture::resolve_conversation_id;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AdapterError {
    #[error("Antigravity binary (agy.exe) not found on system")]
    BinaryNotFound,
    #[error("I/O error during adapter execution: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse quota JSON output: {0}")]
    QuotaParseError(String),
    #[error("CLI process execution failed with status: {0}")]
    ExecutionFailed(String),
}

/// Specifications for launching an agent process inside a terminal surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub cwd: PathBuf,
}

/// Summary of remaining usage quotas across supported model families.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct QuotaSummary {
    pub gemini_weekly_remaining: f32,
    pub gemini_5h_remaining: f32,
    pub third_party_weekly_remaining: f32,
    pub third_party_5h_remaining: f32,
    pub next_reset: Option<String>,
}

/// Abstract adapter trait for hosting CLI agents.
pub trait AgentAdapter: Send + Sync {
    fn name(&self) -> &'static str;
    fn detect_binary(&self) -> Option<PathBuf>;
    fn build_command(
        &self,
        profile: &ProfileConfig,
        working_dir: &Path,
    ) -> Result<LaunchCommand, AdapterError>;
    fn build_resume_command(
        &self,
        conversation_id: &str,
        profile: &ProfileConfig,
        working_dir: &Path,
    ) -> Result<LaunchCommand, AdapterError>;
    fn capture_conversation_id(&self, working_dir: &Path) -> Option<String>;
    fn fetch_quota(&self, profile: &ProfileConfig) -> Result<QuotaSummary, AdapterError>;
}

/// Official adapter for Google Antigravity CLI (`agy`).
pub struct AntigravityAdapter {
    custom_binary: Option<PathBuf>,
}

impl Default for AntigravityAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AntigravityAdapter {
    pub fn new() -> Self {
        Self {
            custom_binary: None,
        }
    }

    pub fn with_binary(binary: PathBuf) -> Self {
        Self {
            custom_binary: Some(binary),
        }
    }
}

impl AgentAdapter for AntigravityAdapter {
    fn name(&self) -> &'static str {
        "Antigravity CLI (agy)"
    }

    fn detect_binary(&self) -> Option<PathBuf> {
        if let Some(custom) = &self.custom_binary {
            if custom.exists() {
                return Some(custom.clone());
            }
        }

        // 1. Official Windows install path: %LOCALAPPDATA%\agy\bin\agy.exe
        if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
            let path = PathBuf::from(local_appdata)
                .join("agy")
                .join("bin")
                .join("agy.exe");
            if path.exists() {
                return Some(path);
            }
        }

        // 2. Fallback path: %USERPROFILE%\.gemini\bin\agy.exe
        if let Some(base_dirs) = directories::BaseDirs::new() {
            let path = base_dirs
                .home_dir()
                .join(".gemini")
                .join("bin")
                .join("agy.exe");
            if path.exists() {
                return Some(path);
            }
        }

        // 3. Search PATH for agy.exe or agy
        if let Ok(path_var) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_var) {
                let candidate_exe = dir.join("agy.exe");
                if candidate_exe.exists() {
                    return Some(candidate_exe);
                }
                let candidate = dir.join("agy");
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }

        None
    }

    fn build_command(
        &self,
        profile: &ProfileConfig,
        working_dir: &Path,
    ) -> Result<LaunchCommand, AdapterError> {
        let binary = self
            .detect_binary()
            .unwrap_or_else(|| PathBuf::from("agy.exe"));

        let mut env = HashMap::new();
        // Enable OSC 7501 statusline emission
        env.insert("AGY_CLI_OSC7501".to_string(), "1".to_string());
        // Clean unattended environment
        env.insert(
            "AGY_CLI_DISABLE_AUTO_UPDATE".to_string(),
            "true".to_string(),
        );
        env.insert("AGY_CLI_HIDE_LOGO".to_string(), "1".to_string());

        // Account override if specified
        if let Some(acc) = &profile.account_override {
            env.insert("AGY_ACCOUNT".to_string(), acc.clone());
        }

        // Mode B storage isolation
        if profile.isolate_storage {
            let dir_str = profile.profile_dir.to_string_lossy().to_string();
            env.insert("HOME".to_string(), dir_str.clone());
            env.insert("USERPROFILE".to_string(), dir_str);
        }

        Ok(LaunchCommand {
            program: binary,
            args: Vec::new(),
            env,
            cwd: working_dir.to_path_buf(),
        })
    }

    fn build_resume_command(
        &self,
        conversation_id: &str,
        profile: &ProfileConfig,
        working_dir: &Path,
    ) -> Result<LaunchCommand, AdapterError> {
        let mut cmd = self.build_command(profile, working_dir)?;
        cmd.args = vec!["--conversation".to_string(), conversation_id.to_string()];
        Ok(cmd)
    }

    fn capture_conversation_id(&self, working_dir: &Path) -> Option<String> {
        let base_gemini_dir = directories::BaseDirs::new()
            .map(|b| b.home_dir().join(".gemini").join("antigravity-cli"))?;
        resolve_conversation_id(working_dir, &base_gemini_dir)
    }

    fn fetch_quota(&self, profile: &ProfileConfig) -> Result<QuotaSummary, AdapterError> {
        let binary = self.detect_binary().ok_or(AdapterError::BinaryNotFound)?;

        let mut cmd = std::process::Command::new(binary);
        cmd.args(["-p", "/usage", "--output-format", "json"]);
        cmd.env("AGY_CLI_DISABLE_AUTO_UPDATE", "true");

        if let Some(acc) = &profile.account_override {
            cmd.env("AGY_ACCOUNT", acc);
        }

        if profile.isolate_storage {
            cmd.env("HOME", &profile.profile_dir);
            cmd.env("USERPROFILE", &profile.profile_dir);
        }

        let output = cmd.output()?;
        if !output.status.success() {
            return Err(AdapterError::ExecutionFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_quota_json(&stdout)
    }
}

/// Parse raw structured JSON from `agy -p "/usage" --output-format json`.
pub fn parse_quota_json(raw: &str) -> Result<QuotaSummary, AdapterError> {
    let root: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| AdapterError::QuotaParseError(e.to_string()))?;

    let mut summary = QuotaSummary::default();

    if let Some(groups) = root.get("groups").and_then(|g| g.as_array()) {
        for group in groups {
            let name = group.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let is_gemini = name.to_lowercase().contains("gemini");
            let is_3p = name.to_lowercase().contains("claude")
                || name.to_lowercase().contains("gpt")
                || name.to_lowercase().contains("3p")
                || name.to_lowercase().contains("third");

            if let Some(windows) = group.get("windows").and_then(|w| w.as_array()) {
                for win in windows {
                    let win_name = win.get("window").and_then(|w| w.as_str()).unwrap_or("");
                    let remaining = win
                        .get("remaining_fraction")
                        .and_then(|f| f.as_f64())
                        .map(|f| f as f32)
                        .unwrap_or(1.0);

                    if summary.next_reset.is_none() {
                        if let Some(reset) = win.get("reset_time").and_then(|r| r.as_str()) {
                            summary.next_reset = Some(reset.to_string());
                        }
                    }

                    if is_gemini {
                        if win_name.contains("week") {
                            summary.gemini_weekly_remaining = remaining;
                        } else if win_name.contains("5h") || win_name.contains("hour") {
                            summary.gemini_5h_remaining = remaining;
                        }
                    } else if is_3p {
                        if win_name.contains("week") {
                            summary.third_party_weekly_remaining = remaining;
                        } else if win_name.contains("5h") || win_name.contains("hour") {
                            summary.third_party_5h_remaining = remaining;
                        }
                    }
                }
            }
        }
    }

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_quota_json_sample() {
        let sample = r#"{
            "groups": [
                {
                    "name": "Gemini Models",
                    "windows": [
                        { "window": "weekly", "remaining_fraction": 0.81, "reset_time": "2026-11-28T12:00:00Z" },
                        { "window": "5h", "remaining_fraction": 0.90, "reset_time": "2026-11-23T20:00:00Z" }
                    ]
                },
                {
                    "name": "Claude and GPT models",
                    "windows": [
                        { "window": "weekly", "remaining_fraction": 0.95, "reset_time": "2026-11-28T12:00:00Z" },
                        { "window": "5h", "remaining_fraction": 1.0, "reset_time": "2026-11-23T20:00:00Z" }
                    ]
                }
            ]
        }"#;

        let quota = parse_quota_json(sample).unwrap();
        assert!((quota.gemini_weekly_remaining - 0.81).abs() < f32::EPSILON);
        assert!((quota.gemini_5h_remaining - 0.90).abs() < f32::EPSILON);
        assert!((quota.third_party_weekly_remaining - 0.95).abs() < f32::EPSILON);
        assert!((quota.third_party_5h_remaining - 1.0).abs() < f32::EPSILON);
        assert_eq!(quota.next_reset, Some("2026-11-28T12:00:00Z".to_string()));
    }

    #[test]
    fn test_build_command_environment() {
        let adapter = AntigravityAdapter::new();
        let profile = ProfileConfig {
            name: "test_profile".to_string(),
            account_override: Some("acc_xyz".to_string()),
            isolate_storage: true,
            profile_dir: PathBuf::from("C:\\AgeMax\\profiles\\test"),
        };

        let cmd = adapter
            .build_command(&profile, Path::new("C:\\Workspace"))
            .unwrap();
        assert_eq!(cmd.env.get("AGY_CLI_OSC7501"), Some(&"1".to_string()));
        assert_eq!(cmd.env.get("AGY_ACCOUNT"), Some(&"acc_xyz".to_string()));
        assert_eq!(
            cmd.env.get("USERPROFILE"),
            Some(&"C:\\AgeMax\\profiles\\test".to_string())
        );
    }

    #[test]
    fn test_build_resume_command() {
        let adapter = AntigravityAdapter::new();
        let profile = ProfileConfig::default_mode_a();
        let cmd = adapter
            .build_resume_command("conv-target-42", &profile, Path::new("C:\\Workspace"))
            .unwrap();

        assert_eq!(cmd.args, vec!["--conversation", "conv-target-42"]);
    }
}
