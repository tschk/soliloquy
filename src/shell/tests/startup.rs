//! The shipped binaries must say why they cannot run instead of exiting 0
//! after doing nothing.

use std::process::Command;

fn run(bin: &str) -> (bool, String) {
    let output = Command::new(bin)
        .env("SOLILOQUY_NO_DAEMON", "1")
        .env("RUST_LOG", "error")
        .output()
        .expect("binary should launch");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn desktop_exits_with_missing_js_engine_error() {
    let (success, stderr) = run(env!("CARGO_BIN_EXE_soliloquy_desktop"));
    assert!(!success, "stderr: {stderr}");
    assert!(
        stderr.contains("V8 runtime is not implemented"),
        "stderr: {stderr}"
    );
}

#[test]
fn shell_exits_with_missing_servo_error() {
    let (success, stderr) = run(env!("CARGO_BIN_EXE_soliloquy_shell"));
    assert!(!success, "stderr: {stderr}");
    assert!(
        stderr.contains("ServoEmbedder is not implemented"),
        "stderr: {stderr}"
    );
}
