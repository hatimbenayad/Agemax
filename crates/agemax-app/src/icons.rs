//! Embedded Lucide vector icon font registration and UI icon helpers.

use agemax_core::status::AgentStatus;
use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, FontId, RichText, TextFormat,
};
use std::sync::Arc;

const LUCIDE_TTF: &[u8] = include_bytes!("../../../assets/fonts/lucide.ttf");
const LUCIDE_FAMILY_NAME: &str = "lucide";

/// Crisp Lucide icon glyphs mapped to their verified codepoints in `assets/fonts/lucide.ttf`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiIcon {
    AlertTriangle,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUpRight,
    Bell,
    Bot,
    ChevronsLeft,
    ChevronsRight,
    Circle,
    CircleCheck,
    CircleX,
    Cloud,
    Columns,
    Command,
    CornerDownLeft,
    Cpu,
    Eye,
    FileText,
    Folder,
    GitBranch,
    Globe,
    Home,
    Keyboard,
    Laptop,
    LayoutGrid,
    Loader,
    Lock,
    LockOpen,
    Maximize,
    Megaphone,
    MemoryStick,
    Menu,
    Minimize,
    MoreHorizontal,
    Pencil,
    Play,
    Plus,
    Radio,
    RotateCw,
    Rows,
    Search,
    Settings,
    Shield,
    Terminal,
    Users,
    Wrench,
    X,
    Zap,
}

impl UiIcon {
    /// Returns the Unicode Private Use Area codepoint inside `lucide.ttf`.
    #[cfg(test)]
    pub const fn codepoint(self) -> char {
        match self {
            Self::AlertTriangle => '\u{E193}',
            Self::ArrowDown => '\u{E042}',
            Self::ArrowLeft => '\u{E048}',
            Self::ArrowRight => '\u{E049}',
            Self::ArrowUpRight => '\u{E04D}',
            Self::Bell => '\u{E059}',
            Self::Bot => '\u{E1BB}',
            Self::ChevronsLeft => '\u{E072}',
            Self::ChevronsRight => '\u{E073}',
            Self::Circle => '\u{E076}',
            Self::CircleCheck => '\u{E226}',
            Self::CircleX => '\u{E084}',
            Self::Cloud => '\u{E088}',
            Self::Columns => '\u{E098}',
            Self::Command => '\u{E09A}',
            Self::CornerDownLeft => '\u{E0A1}',
            Self::Cpu => '\u{E0A9}',
            Self::Eye => '\u{E0BA}',
            Self::FileText => '\u{E0CC}',
            Self::Folder => '\u{E0D7}',
            Self::GitBranch => '\u{E0E2}',
            Self::Globe => '\u{E0E8}',
            Self::Home => '\u{E0F5}',
            Self::Keyboard => '\u{E284}',
            Self::Laptop => '\u{E1CD}',
            Self::LayoutGrid => '\u{E0FF}',
            Self::Loader => '\u{E109}',
            Self::Lock => '\u{E10B}',
            Self::LockOpen => '\u{E10C}',
            Self::Maximize => '\u{E113}',
            Self::Megaphone => '\u{E235}',
            Self::MemoryStick => '\u{E445}',
            Self::Menu => '\u{E115}',
            Self::Minimize => '\u{E11B}',
            Self::MoreHorizontal => '\u{E0B6}',
            Self::Pencil => '\u{E1F9}',
            Self::Play => '\u{E13C}',
            Self::Plus => '\u{E13D}',
            Self::Radio => '\u{E142}',
            Self::RotateCw => '\u{E149}',
            Self::Rows => '\u{E439}',
            Self::Search => '\u{E151}',
            Self::Settings => '\u{E154}',
            Self::Shield => '\u{E158}',
            Self::Terminal => '\u{E181}',
            Self::Users => '\u{E1A4}',
            Self::Wrench => '\u{E1B1}',
            Self::X => '\u{E1B2}',
            Self::Zap => '\u{E1B4}',
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AlertTriangle => "\u{E193}",
            Self::ArrowDown => "\u{E042}",
            Self::ArrowLeft => "\u{E048}",
            Self::ArrowRight => "\u{E049}",
            Self::ArrowUpRight => "\u{E04D}",
            Self::Bell => "\u{E059}",
            Self::Bot => "\u{E1BB}",
            Self::ChevronsLeft => "\u{E072}",
            Self::ChevronsRight => "\u{E073}",
            Self::Circle => "\u{E076}",
            Self::CircleCheck => "\u{E226}",
            Self::CircleX => "\u{E084}",
            Self::Cloud => "\u{E088}",
            Self::Columns => "\u{E098}",
            Self::Command => "\u{E09A}",
            Self::CornerDownLeft => "\u{E0A1}",
            Self::Cpu => "\u{E0A9}",
            Self::Eye => "\u{E0BA}",
            Self::FileText => "\u{E0CC}",
            Self::Folder => "\u{E0D7}",
            Self::GitBranch => "\u{E0E2}",
            Self::Globe => "\u{E0E8}",
            Self::Home => "\u{E0F5}",
            Self::Keyboard => "\u{E284}",
            Self::Laptop => "\u{E1CD}",
            Self::LayoutGrid => "\u{E0FF}",
            Self::Loader => "\u{E109}",
            Self::Lock => "\u{E10B}",
            Self::LockOpen => "\u{E10C}",
            Self::Maximize => "\u{E113}",
            Self::Megaphone => "\u{E235}",
            Self::MemoryStick => "\u{E445}",
            Self::Menu => "\u{E115}",
            Self::Minimize => "\u{E11B}",
            Self::MoreHorizontal => "\u{E0B6}",
            Self::Pencil => "\u{E1F9}",
            Self::Play => "\u{E13C}",
            Self::Plus => "\u{E13D}",
            Self::Radio => "\u{E142}",
            Self::RotateCw => "\u{E149}",
            Self::Rows => "\u{E439}",
            Self::Search => "\u{E151}",
            Self::Settings => "\u{E154}",
            Self::Shield => "\u{E158}",
            Self::Terminal => "\u{E181}",
            Self::Users => "\u{E1A4}",
            Self::Wrench => "\u{E1B1}",
            Self::X => "\u{E1B2}",
            Self::Zap => "\u{E1B4}",
        }
    }
}

/// Dedicated `egui` font family holding embedded Lucide glyphs.
pub fn lucide_font_family() -> FontFamily {
    FontFamily::Name(Arc::from(LUCIDE_FAMILY_NAME))
}

/// Register the embedded `lucide.ttf` font into its own `lucide` font family.
pub fn register_lucide_font(fonts: &mut FontDefinitions) {
    fonts.font_data.insert(
        LUCIDE_FAMILY_NAME.to_owned(),
        Arc::new(FontData::from_static(LUCIDE_TTF)),
    );
    fonts
        .families
        .insert(lucide_font_family(), vec![LUCIDE_FAMILY_NAME.to_owned()]);
}

/// Build `RichText` rendering a standalone Lucide glyph.
pub fn icon_rich(icon: UiIcon, size: f32, color: Option<Color32>) -> RichText {
    let mut rt = RichText::new(icon.as_str())
        .family(lucide_font_family())
        .size(size);
    if let Some(c) = color {
        rt = rt.color(c);
    }
    rt
}

/// Build a centered `LayoutJob` pairing a Lucide icon with proportional label text.
/// When `color` is `None`, `Color32::PLACEHOLDER` lets the hosting widget supply hover/active tints.
pub fn icon_label(
    icon: UiIcon,
    text: &str,
    size: f32,
    color: Option<Color32>,
) -> egui::text::LayoutJob {
    build_icon_text_job(icon, text, size, color, true)
}

/// Convenience helper for standard-sized button/label text with a Lucide icon.
pub fn icon_text(icon: UiIcon, text: &str) -> egui::text::LayoutJob {
    icon_label(icon, text, 12.5, None)
}

/// Convenience helper for compact/small button text with a Lucide icon.
pub fn icon_small(icon: UiIcon, text: &str) -> egui::text::LayoutJob {
    icon_label(icon, text, 11.0, None)
}

/// Build a centered `LayoutJob` with text before a trailing Lucide icon.
pub fn text_then_icon(
    text: &str,
    icon: UiIcon,
    size: f32,
    color: Option<Color32>,
) -> egui::text::LayoutJob {
    build_icon_text_job(icon, text, size, color, false)
}

fn build_icon_text_job(
    icon: UiIcon,
    text: &str,
    size: f32,
    color: Option<Color32>,
    icon_first: bool,
) -> egui::text::LayoutJob {
    let fg = color.unwrap_or(Color32::PLACEHOLDER);
    let icon_fmt = TextFormat {
        font_id: FontId::new(size, lucide_font_family()),
        color: fg,
        valign: egui::Align::Center,
        ..Default::default()
    };
    let text_fmt = TextFormat {
        font_id: FontId::new(size, FontFamily::Proportional),
        color: fg,
        valign: egui::Align::Center,
        ..Default::default()
    };

    let mut job = egui::text::LayoutJob::default();
    if icon_first {
        job.append(icon.as_str(), 0.0, icon_fmt);
        if !text.is_empty() {
            job.append(" ", 0.0, text_fmt.clone());
            job.append(text, 0.0, text_fmt);
        }
    } else {
        if !text.is_empty() {
            job.append(text, 0.0, text_fmt.clone());
            job.append(" ", 0.0, text_fmt);
        }
        job.append(icon.as_str(), 0.0, icon_fmt);
    }
    job
}

/// Map an `AgentStatus` to its crisp Lucide icon.
pub const fn status_icon(status: AgentStatus) -> UiIcon {
    match status {
        AgentStatus::Idle => UiIcon::Circle,
        AgentStatus::Working => UiIcon::Zap,
        AgentStatus::Thinking => UiIcon::Loader,
        AgentStatus::RunningTool => UiIcon::Wrench,
        AgentStatus::WaitingForInput => UiIcon::AlertTriangle,
        AgentStatus::Done => UiIcon::CircleCheck,
        AgentStatus::Failed => UiIcon::CircleX,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lucide_font_embedded_and_registered() {
        assert!(!LUCIDE_TTF.is_empty());
        let mut defs = FontDefinitions::default();
        register_lucide_font(&mut defs);
        assert!(defs.font_data.contains_key(LUCIDE_FAMILY_NAME));
        assert!(defs.families.contains_key(&lucide_font_family()));
    }

    #[test]
    fn test_status_icons_match_codepoints() {
        for status in [
            AgentStatus::Idle,
            AgentStatus::Working,
            AgentStatus::Thinking,
            AgentStatus::RunningTool,
            AgentStatus::WaitingForInput,
            AgentStatus::Done,
            AgentStatus::Failed,
        ] {
            let icon = status_icon(status);
            assert_eq!(icon.as_str().chars().next(), Some(icon.codepoint()));
        }
    }
}
