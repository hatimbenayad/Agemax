use serde::{Deserialize, Serialize};

/// Built-in visual theme presets selectable from Settings or `config.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemePreset {
    MidnightDark,
    HighContrast,
    SoftLight,
}

/// Resolved RGB palette ready for zero-allocation hot-path terminal rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedTheme {
    pub background: (u8, u8, u8),
    pub foreground: (u8, u8, u8),
    pub cursor: (u8, u8, u8),
    pub active_border: (u8, u8, u8),
    pub inactive_border: (u8, u8, u8),
    pub ring_waiting: (u8, u8, u8),
    pub ring_done: (u8, u8, u8),
    pub ring_failed: (u8, u8, u8),
}

impl ResolvedTheme {
    /// Perceived luminance (0..=255) of the primary background color.
    pub fn background_luminance(&self) -> u32 {
        let (r, g, b) = self.background;
        (r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000
    }

    pub fn is_light_background(&self) -> bool {
        self.background_luminance() >= 128
    }
}

/// Hex color representation for terminal and UI elements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub background: String,
    pub foreground: String,
    pub cursor: String,
    pub active_border: String,
    pub inactive_border: String,
    pub ring_waiting: String,
    pub ring_done: String,
    pub ring_failed: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self::from_preset(ThemePreset::MidnightDark)
    }
}

impl ThemeConfig {
    /// Build a complete `ThemeConfig` from one of the built-in presets.
    pub fn from_preset(preset: ThemePreset) -> Self {
        match preset {
            ThemePreset::MidnightDark => Self {
                background: "#0F1117".to_string(),
                foreground: "#DCE0E8".to_string(),
                cursor: "#E6EBF5".to_string(),
                active_border: "#466EC8".to_string(),
                inactive_border: "#20232D".to_string(),
                ring_waiting: "#F5AF2D".to_string(),
                ring_done: "#2DC86E".to_string(),
                ring_failed: "#EB3C3C".to_string(),
            },
            ThemePreset::HighContrast => Self {
                background: "#080A0E".to_string(),
                foreground: "#FFFFFF".to_string(),
                cursor: "#FFD866".to_string(),
                active_border: "#5CA8FF".to_string(),
                inactive_border: "#404658".to_string(),
                ring_waiting: "#FFB800".to_string(),
                ring_done: "#30E87A".to_string(),
                ring_failed: "#FF4D4D".to_string(),
            },
            ThemePreset::SoftLight => Self {
                background: "#F5F7FA".to_string(),
                foreground: "#1E222E".to_string(),
                cursor: "#2B59C3".to_string(),
                active_border: "#2B59C3".to_string(),
                inactive_border: "#D0D7E3".to_string(),
                ring_waiting: "#D97706".to_string(),
                ring_done: "#15803D".to_string(),
                ring_failed: "#DC2626".to_string(),
            },
        }
    }

    /// Parse all configured hex colors into `(r, g, b)` tuples with safe fallbacks.
    pub fn resolved_rgb(&self) -> ResolvedTheme {
        let defaults = Self::default();
        ResolvedTheme {
            background: resolve_hex_or(&self.background, &defaults.background, (15, 17, 23)),
            foreground: resolve_hex_or(&self.foreground, &defaults.foreground, (220, 224, 232)),
            cursor: resolve_hex_or(&self.cursor, &defaults.cursor, (230, 235, 245)),
            active_border: resolve_hex_or(
                &self.active_border,
                &defaults.active_border,
                (70, 110, 200),
            ),
            inactive_border: resolve_hex_or(
                &self.inactive_border,
                &defaults.inactive_border,
                (32, 35, 45),
            ),
            ring_waiting: resolve_hex_or(
                &self.ring_waiting,
                &defaults.ring_waiting,
                (245, 175, 45),
            ),
            ring_done: resolve_hex_or(&self.ring_done, &defaults.ring_done, (45, 200, 110)),
            ring_failed: resolve_hex_or(&self.ring_failed, &defaults.ring_failed, (235, 60, 60)),
        }
    }
}

fn resolve_hex_or(value: &str, default_hex: &str, hardcoded: (u8, u8, u8)) -> (u8, u8, u8) {
    parse_hex_color(value)
        .or_else(|| parse_hex_color(default_hex))
        .unwrap_or(hardcoded)
}

/// Parse a `#RRGGBB` hex string into `(r, g, b)`.
pub fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8)> {
    let clean = hex.trim().strip_prefix('#').unwrap_or(hex.trim());
    if clean.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&clean[0..2], 16).ok()?;
    let g = u8::from_str_radix(&clean[2..4], 16).ok()?;
    let b = u8::from_str_radix(&clean[4..6], 16).ok()?;
    Some((r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color("#0F1117"), Some((15, 17, 23)));
        assert_eq!(parse_hex_color("F5AF2D"), Some((245, 175, 45)));
        assert_eq!(parse_hex_color("invalid"), None);
    }
}
