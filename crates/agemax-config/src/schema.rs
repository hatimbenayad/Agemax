use crate::keybindings::KeybindingsConfig;
use crate::theme::ThemeConfig;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("I/O error reading configuration: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse TOML configuration: {0}")]
    Parse(#[from] toml::de::Error),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub font_size: f32,
    pub default_shell: Option<String>,
    pub auto_update_check: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            font_size: 14.0,
            default_shell: None,
            auto_update_check: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomCommand {
    pub name: String,
    pub description: Option<String>,
    pub command: String,
    pub working_dir: Option<PathBuf>,
    pub keybinding: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AgeMaxConfig {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub keybindings: KeybindingsConfig,
    #[serde(default)]
    pub commands: Vec<CustomCommand>,
}

impl AgeMaxConfig {
    /// Load configuration from a specified TOML path.
    pub fn load_from_path(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load default configuration, creating a template file if it does not yet exist.
    pub fn load_or_create_default() -> Self {
        let path = Self::default_path();
        if path.exists() {
            Self::load_from_path(&path).unwrap_or_default()
        } else {
            let default_config = Self::default();
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(content) = toml::to_string_pretty(&default_config) {
                let _ = std::fs::write(&path, content);
            }
            default_config
        }
    }

    /// Default configuration path under `%APPDATA%\AgeMax\config.toml`.
    pub fn default_path() -> PathBuf {
        directories::ProjectDirs::from("com", "agemax", "AgeMax")
            .map(|p| p.config_dir().join("config.toml"))
            .unwrap_or_else(|| PathBuf::from(".agemax").join("config.toml"))
    }

    /// Merge custom commands from a workspace-local `agemax.toml` if present.
    pub fn merge_workspace_config(&mut self, workspace_dir: &Path) {
        let ws_config_path = workspace_dir.join("agemax.toml");
        if ws_config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&ws_config_path) {
                if let Ok(ws_cfg) = toml::from_str::<AgeMaxConfig>(&content) {
                    for cmd in ws_cfg.commands {
                        if !self.commands.iter().any(|c| c.name == cmd.name) {
                            self.commands.push(cmd);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sample_config() {
        let toml_str = r##"
            [general]
            font_size = 15.0
            auto_update_check = false

            [theme]
            background = "#1a1b26"
            active_border = "#7aa2f7"

            [[commands]]
            name = "Run Tests"
            command = "cargo test"
            description = "Execute workspace unit tests"
        "##;

        let cfg: AgeMaxConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.general.font_size, 15.0);
        assert_eq!(cfg.theme.background, "#1a1b26");
        assert_eq!(cfg.commands.len(), 1);
        assert_eq!(cfg.commands[0].name, "Run Tests");
    }

    #[test]
    fn test_merge_workspace_config() {
        let temp_dir =
            std::env::temp_dir().join(format!("agemax_test_ws_cfg_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let ws_toml = r#"
            [[commands]]
            name = "Local Build"
            command = "cargo build"
        "#;
        std::fs::write(temp_dir.join("agemax.toml"), ws_toml).unwrap();

        let mut config = AgeMaxConfig::default();
        config.merge_workspace_config(&temp_dir);

        assert!(config.commands.iter().any(|c| c.name == "Local Build"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
