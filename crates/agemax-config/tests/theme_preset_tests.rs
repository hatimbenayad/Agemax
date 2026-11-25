use agemax_config::theme::{ThemeConfig, ThemePreset};

#[test]
fn test_theme_presets_and_resolved_rgb_fallbacks() {
    let midnight = ThemeConfig::from_preset(ThemePreset::MidnightDark);
    let hc = ThemeConfig::from_preset(ThemePreset::HighContrast);
    let light = ThemeConfig::from_preset(ThemePreset::SoftLight);
    let cyber = ThemeConfig::from_preset(ThemePreset::Cyberpunk);
    let green = ThemeConfig::from_preset(ThemePreset::TerminalGreen);
    let dracula = ThemeConfig::from_preset(ThemePreset::Dracula);

    assert_eq!(ThemePreset::all().len(), 6);
    assert_ne!(midnight.background, light.background);
    assert_ne!(cyber.active_border, midnight.active_border);
    assert_ne!(green.foreground, midnight.foreground);
    assert_ne!(dracula.background, midnight.background);
    assert!(!midnight.resolved_rgb().is_light_background());
    assert!(!cyber.resolved_rgb().is_light_background());
    assert!(!green.resolved_rgb().is_light_background());
    assert!(!dracula.resolved_rgb().is_light_background());
    assert!(light.resolved_rgb().is_light_background());
    assert_eq!(hc.resolved_rgb().foreground, (255, 255, 255));

    // Malformed hex must fall back safely to default color rather than panicking
    let malformed = ThemeConfig {
        background: "not-a-color".to_string(),
        foreground: "#XYZ".to_string(),
        ..Default::default()
    };
    let resolved = malformed.resolved_rgb();
    assert_eq!(resolved.background, (15, 17, 23));
    assert_eq!(resolved.foreground, (220, 224, 232));
}
