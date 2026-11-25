use agemax_config::safety::{
    display_clean_path, resolve_startup_directory, validate_workspace_path, PathSafetyError,
};
use agemax_config::schema::{AgeMaxConfig, GeneralConfig};
use std::path::{Path, PathBuf};

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("agemax-safety-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn test_allowed_root_blocks_outside_and_traversal() {
    let temp = TestDir::new();
    let safe_root = temp.path().join("safe-projects");
    let sub_project = safe_root.join("app-one");
    let outside_dir = temp.path().join("outside-secrets");

    std::fs::create_dir_all(&sub_project).unwrap();
    std::fs::create_dir_all(&outside_dir).unwrap();

    // Unconfigured lock allows valid existing directories
    assert!(validate_workspace_path(&outside_dir, None).is_ok());

    // Allowed root itself and child subfolders pass
    assert!(validate_workspace_path(&safe_root, Some(&safe_root)).is_ok());
    assert!(validate_workspace_path(&sub_project, Some(&safe_root)).is_ok());

    // Sibling folder outside allowed_root is rejected
    let err = validate_workspace_path(&outside_dir, Some(&safe_root)).unwrap_err();
    assert!(matches!(err, PathSafetyError::OutsideAllowedRoot { .. }));

    // Lexically crafted .. traversal out of allowed_root is rejected
    let traversal = safe_root.join("..").join("outside-secrets");
    let err_trav = validate_workspace_path(&traversal, Some(&safe_root)).unwrap_err();
    assert!(matches!(
        err_trav,
        PathSafetyError::OutsideAllowedRoot { .. }
    ));
}

#[test]
fn test_startup_dir_resolves_default_and_clamps_to_allowed_root() {
    let temp = TestDir::new();
    let safe_root = temp.path().join("sandbox-root");
    let default_sub = safe_root.join("starter");
    let outside_dir = temp.path().join("random-cwd");

    std::fs::create_dir_all(&default_sub).unwrap();
    std::fs::create_dir_all(&outside_dir).unwrap();

    let cfg = GeneralConfig {
        default_workspace_dir: Some(default_sub.clone()),
        allowed_root_dir: Some(safe_root.clone()),
        ..Default::default()
    };

    // Unapproved startup dir clamps safely inside allowed_root / default_workspace_dir without \\?\ prefix
    let resolved_blocked = resolve_startup_directory(Some(&outside_dir), &cfg);
    assert!(!resolved_blocked.to_string_lossy().starts_with(r"\\?\"));
    assert_eq!(
        display_clean_path(&resolved_blocked),
        display_clean_path(&std::fs::canonicalize(&default_sub).unwrap())
    );

    // Explicit approved subfolder succeeds
    let approved_sub = safe_root.join("explicit");
    std::fs::create_dir_all(&approved_sub).unwrap();
    let resolved_ok = resolve_startup_directory(Some(&approved_sub), &cfg);
    assert!(!resolved_ok.to_string_lossy().starts_with(r"\\?\"));
    assert_eq!(
        display_clean_path(&resolved_ok),
        display_clean_path(&std::fs::canonicalize(&approved_sub).unwrap())
    );
}

#[test]
fn test_general_safety_config_defaults_and_roundtrip() {
    let cfg = AgeMaxConfig::default();
    assert_eq!(cfg.general.default_workspace_dir, None);
    assert_eq!(cfg.general.allowed_root_dir, None);
    assert!(!cfg.general.enforce_agy_sandbox);

    let temp = TestDir::new();
    let path = temp.path().join("config.toml");

    let mut custom = AgeMaxConfig::default();
    custom.general.default_workspace_dir = Some(PathBuf::from(r"C:\Safe\Default"));
    custom.general.allowed_root_dir = Some(PathBuf::from(r"C:\Safe"));
    custom.general.enforce_agy_sandbox = true;
    custom.save_to_path(&path).unwrap();

    let loaded = AgeMaxConfig::load_from_path(&path).unwrap();
    assert_eq!(
        loaded.general.default_workspace_dir,
        Some(PathBuf::from(r"C:\Safe\Default"))
    );
    assert_eq!(
        loaded.general.allowed_root_dir,
        Some(PathBuf::from(r"C:\Safe"))
    );
    assert!(loaded.general.enforce_agy_sandbox);
}
