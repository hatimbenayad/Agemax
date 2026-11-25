use agemax_core::model::TerminalSize;
use agemax_term::backend::{
    parse_wsl_list_output, LocalConPtyBackend, PtySystemBackend, SshPtyBackend, WslPtyBackend,
};
use std::collections::HashMap;
use std::path::Path;

#[test]
fn test_pty_system_backends_build_configs() {
    let size = TerminalSize::new(100, 30);

    let local = LocalConPtyBackend;
    let local_cfg =
        local.build_pty_config(Path::new("."), Some("powershell"), size, HashMap::new());
    assert_eq!(local.kind_label(), "Local");
    assert_eq!(local_cfg.cols, 100);
    assert_eq!(local_cfg.rows, 30);

    let wsl = WslPtyBackend {
        distro: Some("Ubuntu-24.04".to_string()),
        linux_cwd: Some("~/work".to_string()),
    };
    let wsl_cfg = wsl.build_pty_config(Path::new("."), None, size, HashMap::new());
    assert_eq!(wsl.kind_label(), "WSL");
    assert!(wsl_cfg.args.contains(&"-d".to_string()));
    assert!(wsl_cfg.args.contains(&"Ubuntu-24.04".to_string()));
    assert!(wsl_cfg.args.contains(&"--cd".to_string()));

    let ssh = SshPtyBackend {
        target: "hatim@devbox".to_string(),
        port: Some(2222),
        remote_dir: Some("/opt/repo".to_string()),
    };
    let ssh_cfg = ssh.build_pty_config(Path::new("."), None, size, HashMap::new());
    assert_eq!(ssh.kind_label(), "SSH");
    assert!(ssh_cfg.args.contains(&"-t".to_string()));
    assert!(ssh_cfg.args.contains(&"-p".to_string()));
    assert!(ssh_cfg.args.contains(&"2222".to_string()));
    assert!(ssh_cfg.args.contains(&"hatim@devbox".to_string()));
}

#[test]
fn test_parse_wsl_distro_list_output() {
    // Simulated UTF-16LE output from `wsl.exe --list --quiet`
    let utf16: Vec<u16> = "Ubuntu-24.04\r\nDebian\r\ndocker-desktop\r\n"
        .encode_utf16()
        .collect();
    let bytes: Vec<u8> = utf16.iter().flat_map(|u| u.to_le_bytes()).collect();
    let distros = parse_wsl_list_output(&bytes);
    assert_eq!(distros, vec!["Ubuntu-24.04", "Debian", "docker-desktop"]);
}
