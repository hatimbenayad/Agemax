//! Embedded Lucide vector icon font registration.

use eframe::egui::{FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

const LUCIDE_TTF: &[u8] = include_bytes!("../../../assets/fonts/lucide.ttf");
const LUCIDE_FAMILY_NAME: &str = "lucide";

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
}
