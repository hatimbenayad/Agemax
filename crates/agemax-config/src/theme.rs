use serde::{Deserialize, Serialize};

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
        Self {
            background: "#0F1117".to_string(),
            foreground: "#DCE0E8".to_string(),
            cursor: "#E6EBF5".to_string(),
            active_border: "#466EC8".to_string(),
            inactive_border: "#20232D".to_string(),
            ring_waiting: "#F5AF2D".to_string(),
            ring_done: "#2DC86E".to_string(),
            ring_failed: "#EB3C3C".to_string(),
        }
    }
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
