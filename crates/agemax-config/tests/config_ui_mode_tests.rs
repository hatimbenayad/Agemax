use agemax_config::keybindings::{KeyChord, KeyCode, KeybindingsConfig};
use agemax_config::schema::{AgeMaxConfig, GeneralConfig, UiMode};

#[test]
fn test_ui_mode_defaults_to_guided_and_parses_legacy_toml() {
    let legacy_toml = r#"
        [general]
        font_size = 15.0
    "#;
    let cfg: AgeMaxConfig = toml::from_str(legacy_toml).expect("legacy TOML must deserialize");
    assert_eq!(cfg.general.ui_mode, UiMode::Guided);
    assert!(!cfg.general.sidebar_collapsed);
    assert!(cfg.general.effective_show_toolbar());
    assert!(cfg.general.effective_show_pane_headers());
    assert!(cfg.general.effective_show_shortcut_hints());

    let minimal_toml = r#"
        [general]
        ui_mode = "minimal"
    "#;
    let min_cfg: AgeMaxConfig =
        toml::from_str(minimal_toml).expect("minimal UI mode TOML must deserialize");
    assert_eq!(min_cfg.general.ui_mode, UiMode::Minimal);
    assert!(!min_cfg.general.effective_show_toolbar());
    assert!(!min_cfg.general.effective_show_pane_headers());
    assert!(!min_cfg.general.effective_show_shortcut_hints());
}

#[test]
fn test_ui_mode_granular_overrides_win_over_mode_preset() {
    let mut cfg = GeneralConfig {
        ui_mode: UiMode::Minimal,
        show_toolbar: Some(true),
        show_pane_headers: None,
        show_shortcut_hints: Some(false),
        ..Default::default()
    };

    assert!(cfg.effective_show_toolbar());
    assert!(!cfg.effective_show_pane_headers());
    assert!(!cfg.effective_show_shortcut_hints());

    cfg.ui_mode = UiMode::Guided;
    cfg.show_toolbar = Some(false);
    assert!(!cfg.effective_show_toolbar());
    assert!(cfg.effective_show_pane_headers());
}

#[test]
fn test_config_save_and_reload_roundtrip() {
    let temp_dir = std::env::temp_dir().join(format!("agemax_cfg_rt_{}", uuid::Uuid::new_v4()));
    let path = temp_dir.join("nested").join("config.toml");

    let mut cfg = AgeMaxConfig::default();
    cfg.general.ui_mode = UiMode::Minimal;
    cfg.general.show_toolbar = Some(true);
    cfg.general.sidebar_collapsed = true;
    cfg.keybindings.new_tab = "Ctrl+Shift+N".to_string();

    cfg.save_to_path(&path)
        .expect("save_to_path should create parent dirs and persist TOML");

    let loaded = AgeMaxConfig::load_from_path(&path).expect("reload persisted config");
    assert_eq!(loaded, cfg);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_keybindings_preserve_defaults_and_parse_chords() {
    let defaults = KeybindingsConfig::default();
    assert_eq!(defaults.split_horizontal, "Ctrl+Shift+D");
    assert_eq!(defaults.split_vertical, "Ctrl+Shift+R");
    assert_eq!(defaults.new_tab, "Ctrl+Shift+T");

    let chord = KeyChord::parse(&defaults.new_tab).expect("new_tab chord must parse");
    assert!(chord.ctrl);
    assert!(chord.shift);
    assert!(!chord.alt);
    assert_eq!(chord.key, KeyCode::Char('T'));
    assert!(chord.matches(true, true, false, KeyCode::Char('T')));
    assert!(!chord.matches(true, false, false, KeyCode::Char('T')));

    let toggle_sidebar =
        KeyChord::parse("ctrl + b").expect("lowercase/spaced key chord should parse");
    assert!(toggle_sidebar.matches(true, false, false, KeyCode::Char('B')));

    let zoom = KeyChord::parse(&defaults.zoom_pane).expect("Ctrl+Shift+Enter must parse");
    assert!(zoom.matches(true, true, false, KeyCode::Enter));

    assert!(KeyChord::parse("").is_none());
    assert!(KeyChord::parse("Ctrl+Shift+").is_none());
}
