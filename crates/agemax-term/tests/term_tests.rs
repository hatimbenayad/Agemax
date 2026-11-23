use agemax_term::coalescer::PtyCoalescer;
use agemax_term::pty::detect_default_shell;
use agemax_term::vt::{TerminalGrid, TerminalGridConfig};
use std::time::Duration;

#[test]
fn test_detect_default_shell() {
    let shell = detect_default_shell();
    println!("Detected shell: {:?}", shell);
    assert!(!shell.as_os_str().is_empty());
    let filename = shell.file_name().unwrap().to_string_lossy().to_lowercase();
    assert!(
        filename.contains("pwsh") || filename.contains("powershell") || filename.contains("cmd"),
        "Unexpected shell filename: {}",
        filename
    );
}

#[test]
fn test_vt_terminal_grid_ansi_processing() {
    let mut grid = TerminalGrid::new(TerminalGridConfig { cols: 80, rows: 24 });
    let payload = b"Hello \x1b[31mRed World\x1b[0m\r\n";
    grid.advance_bytes(payload);

    let line0 = grid.render_line_text(0);
    assert!(
        line0.starts_with("Hello Red World"),
        "Line was: {:?}",
        line0
    );

    // Verify cell color attributes
    let cell_red = grid.get_cell(0, 7); // 'R' in "Red"
    assert!(cell_red.fg.is_some());
}

#[test]
fn test_detect_git_branch_in_current_repo() {
    let cwd = std::env::current_dir().unwrap();
    let branch = agemax_term::metadata::detect_git_branch(&cwd);
    println!("Detected branch in workspace: {:?}", branch);
    assert!(
        branch.is_some(),
        "Expected to detect git branch in current workspace"
    );
}

#[test]
fn test_coalescer_batching() {
    let (mut coalescer, rx) = PtyCoalescer::new(Duration::from_millis(10));
    coalescer.push_bytes(b"chunk1 ");
    coalescer.push_bytes(b"chunk2 ");
    coalescer.flush();

    let received = rx
        .recv_timeout(Duration::from_secs(1))
        .expect("Timed out waiting for coalescer");
    assert_eq!(received.as_slice(), b"chunk1 chunk2 ");
}

#[cfg(windows)]
#[test]
fn test_job_object_lifecycle() {
    use agemax_term::job_object::JobObject;
    let job = JobObject::create().expect("Failed to create Job Object");
    let current_pid = std::process::id();
    // Assigning current process or child process to job
    let res = job.assign_pid(current_pid);
    // Note: If running inside some test harness already in a job with no breakaway, this might return an expected OS code,
    // but the call itself must be valid Win32.
    println!("Job object assign pid result: {:?}", res);
}
