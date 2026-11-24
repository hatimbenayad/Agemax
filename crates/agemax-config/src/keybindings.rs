use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeybindingsConfig {
    pub new_tab: String,
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
            new_tab: "Ctrl+Shift+T".to_string(),
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

/// Platform-independent key identifier for matching shortcut chords.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    Char(char),
    Enter,
    Escape,
    Tab,
    Backspace,
    Space,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
}

/// Parsed keyboard shortcut chord (`Ctrl+Shift+T`, `Ctrl+B`, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyChord {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub key: KeyCode,
}

impl KeyChord {
    /// Parse a human-readable key chord string such as `"Ctrl+Shift+T"` or `"Ctrl+B"`.
    pub fn parse(spec: &str) -> Option<Self> {
        let trimmed = spec.trim();
        if trimmed.is_empty() {
            return None;
        }

        let mut ctrl = false;
        let mut shift = false;
        let mut alt = false;
        let mut key = None;

        for raw_part in trimmed.split('+') {
            let token = raw_part.trim().to_ascii_lowercase();
            if token.is_empty() {
                return None;
            }
            match token.as_str() {
                "ctrl" | "control" => ctrl = true,
                "shift" => shift = true,
                "alt" => alt = true,
                _ => {
                    if key.is_some() {
                        return None;
                    }
                    key = parse_key_token(&token);
                    key?;
                }
            }
        }

        Some(Self {
            ctrl,
            shift,
            alt,
            key: key?,
        })
    }

    /// Check whether pressed modifiers and key match this parsed chord.
    pub fn matches(&self, ctrl: bool, shift: bool, alt: bool, key: KeyCode) -> bool {
        self.ctrl == ctrl && self.shift == shift && self.alt == alt && self.key == key
    }
}

fn parse_key_token(token: &str) -> Option<KeyCode> {
    match token {
        "enter" | "return" => Some(KeyCode::Enter),
        "esc" | "escape" => Some(KeyCode::Escape),
        "tab" => Some(KeyCode::Tab),
        "backspace" | "bksp" => Some(KeyCode::Backspace),
        "space" => Some(KeyCode::Space),
        "up" | "arrowup" => Some(KeyCode::ArrowUp),
        "down" | "arrowdown" => Some(KeyCode::ArrowDown),
        "left" | "arrowleft" => Some(KeyCode::ArrowLeft),
        "right" | "arrowright" => Some(KeyCode::ArrowRight),
        _ => {
            let mut chars = token.chars();
            let ch = chars.next()?;
            if chars.next().is_none() && ch.is_ascii_alphanumeric() {
                Some(KeyCode::Char(ch.to_ascii_uppercase()))
            } else {
                None
            }
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
