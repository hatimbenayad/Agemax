use agemax_term::pty::{available_shells, resolve_shell_program};

#[test]
fn test_resolve_shell_program_variants() {
    let auto = resolve_shell_program(None);
    assert!(!auto.as_os_str().is_empty());

    let cmd = resolve_shell_program(Some("cmd"));
    assert!(
        cmd.to_string_lossy().to_lowercase().contains("cmd"),
        "Expected cmd shell, got {:?}",
        cmd
    );

    let ps = resolve_shell_program(Some("powershell"));
    assert!(
        ps.to_string_lossy().to_lowercase().contains("powershell"),
        "Expected powershell shell, got {:?}",
        ps
    );

    let choices = available_shells();
    assert!(choices.iter().any(|(id, _)| *id == "auto"));
    assert!(choices.iter().any(|(id, _)| *id == "cmd"));
}
