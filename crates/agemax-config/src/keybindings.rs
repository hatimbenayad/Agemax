use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeybindingsConfig {
    pub split_horizontal: String,
    pub split_vertical: String,
    pub close_pane: String,
    pub zoom_pane: String,
    pub toggle_sidebar: String,
    pub command_palette: String,
    pub jump_unread: String,
    pub toggle_notifications: String,
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            split_horizontal: "Ctrl+Shift+D".to_string(),
            split_vertical: "Ctrl+Shift+R".to_string(),
            close_pane: "Ctrl+Shift+W".to_string(),
            zoom_pane: "Ctrl+Shift+Enter".to_string(),
            toggle_sidebar: "Ctrl+B".to_string(),
            command_palette: "Ctrl+Shift+P".to_string(),
            jump_unread: "Ctrl+Shift+U".to_string(),
            toggle_notifications: "Ctrl+Shift+N".to_string(),
        }
    }
}

/// Verify that configured keybindings do not steal essential terminal interrupt keys.
pub fn validate_keybinding_safety(binding: &str) -> bool {
    let normalized = binding.trim().to_lowercase().replace(' ', "");
    !matches!(
        normalized.as_str(),
        "ctrl+c" | "ctrl+d" | "ctrl+z" | "ctrl+l"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_keybindings_are_safe() {
        let defaults = KeybindingsConfig::default();
        assert!(validate_keybinding_safety(&defaults.split_horizontal));
        assert!(validate_keybinding_safety(&defaults.command_palette));
        assert!(validate_keybinding_safety(&defaults.toggle_sidebar));
    }

    #[test]
    fn test_reject_dangerous_bindings() {
        assert!(!validate_keybinding_safety("Ctrl+C"));
        assert!(!validate_keybinding_safety("ctrl + c"));
        assert!(!validate_keybinding_safety("Ctrl+D"));
        assert!(!validate_keybinding_safety("Ctrl+Z"));
    }
}
