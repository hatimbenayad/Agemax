use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn test_ico_asset_and_packaging_manifests_exist() {
    let root = repo_root();
    let ico_path = root.join("assets").join("agemax.ico");
    let bytes = std::fs::read(&ico_path)
        .unwrap_or_else(|e| panic!("Expected valid .ico at {}: {}", ico_path.display(), e));

    // Windows ICO header: reserved = 0, type = 1, image_count >= 3
    assert!(bytes.len() > 22, "ICO file too small");
    assert_eq!(&bytes[0..4], &[0, 0, 1, 0], "Invalid ICO magic bytes");
    let count = u16::from_le_bytes([bytes[4], bytes[5]]);
    assert!(count >= 3, "Expected at least 3 icon sizes, got {count}");

    let pkg_dir = root.join("packaging").join("windows");
    for rel in [
        "agemax.iss",
        "open-in-agemax-install.ps1",
        "open-in-agemax-uninstall.ps1",
        "winget/hatimbenayad.AgeMax.yaml",
        "winget/hatimbenayad.AgeMax.installer.yaml",
        "winget/hatimbenayad.AgeMax.locale.en-US.yaml",
    ] {
        let p = pkg_dir.join(rel);
        assert!(p.exists(), "Missing packaging file: {}", p.display());
    }
}
