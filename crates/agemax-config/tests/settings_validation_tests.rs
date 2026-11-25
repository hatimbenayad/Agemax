use agemax_config::keybindings::KeybindingsConfig;
use agemax_config::schema::AgeMaxConfig;

#[test]
fn test_general_notification_and_onboarding_defaults() {
    let cfg: AgeMaxConfig = toml::from_str("").expect("empty config should use defaults");
    assert!(cfg.general.enable_toast_notifications);
    assert!(cfg.general.enable_taskbar_flash);
    assert!(cfg.general.enable_sound_notifications);
    assert!(!cfg.general.show_agent_mascots);
    assert!(!cfg.general.enable_mica_backdrop);
    assert!(cfg.general.enable_onboarding_banner);

    // Existing saved TOMLs with explicit `show_agent_mascots = true` preserve user preference
    let legacy_mascot: AgeMaxConfig =
        toml::from_str("[general]\nshow_agent_mascots = true\n").expect("valid TOML");
    assert!(legacy_mascot.general.show_agent_mascots);
}

#[test]
fn test_validate_all_keybindings_rejects_unsafe_and_invalid() {
    let mut kb = KeybindingsConfig::default();
    assert!(kb.validate_all().is_ok());

    kb.command_palette = "Ctrl+K".to_string();
    assert!(kb.validate_all().is_ok());

    kb.close_pane = "Ctrl+C".to_string();
    assert!(kb.validate_all().is_err());

    kb.close_pane = "Invalid++Key".to_string();
    assert!(kb.validate_all().is_err());
}
