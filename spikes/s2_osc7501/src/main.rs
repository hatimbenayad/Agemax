use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::Read;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn main() {
    let pty_system = NativePtySystem::default();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("Failed to open PTY");

    let mut cmd = CommandBuilder::new("cmd.exe");
    cmd.args(["/C", "echo \x1b]7501;status=working\x07 && echo \x1b]7501;status=waiting;prompt=approval\x1b\\ && echo NORMAL_TEXT_MARKER"]);

    let mut child = pair.slave.spawn_command(cmd).expect("Failed to spawn child");
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().expect("Failed to get reader");
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let mut chunk = [0u8; 1024];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    let _ = tx.send(chunk[..n].to_vec());
                }
                Err(_) => break,
            }
        }
    });

    let _ = child.wait();
    thread::sleep(Duration::from_millis(500));
    let mut total_output = Vec::new();
    while let Ok(chunk) = rx.try_recv() {
        total_output.extend_from_slice(&chunk);
    }

    // Inspect escape sequences
    println!("Byte slice contains 7501:");
    let mut i = 0;
    while i < total_output.len() {
        if total_output[i..].starts_with(b"\x1b]") {
            let start = i;
            let mut end = i + 2;
            while end < total_output.len() && total_output[end] != 0x07 && !total_output[end..].starts_with(b"\x1b\\") {
                end += 1;
            }
            if end < total_output.len() {
                if total_output[end] == 0x07 {
                    end += 1;
                } else if total_output[end..].starts_with(b"\x1b\\") {
                    end += 2;
                }
            }
            println!("Captured OSC: {:?}", String::from_utf8_lossy(&total_output[start..end]));
            i = end;
        } else {
            i += 1;
        }
    }
}
