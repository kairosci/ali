#[cfg(target_os = "linux")]
#[test]
fn native_gui_opens_renders_and_closes() {
    use std::{
        path::PathBuf,
        process::{Command, Stdio},
        thread,
        time::{Duration, Instant},
    };

    if std::env::var_os("DISPLAY").is_none() {
        eprintln!("skipping native GUI smoke test because DISPLAY is unavailable");
        return;
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let temporary = tempfile::tempdir().expect("temporary GUI fixture directory is available");
    let source = temporary.path().join("gui-private.txt");
    let redacted = temporary.path().join("gui-private.redacted.txt");
    std::fs::write(&source, "GUI local scan for client@example.com.\n")
        .expect("synthetic GUI document is written");
    let mut child = Command::new(env!("CARGO_BIN_EXE_ali"))
        .arg("--config-dir")
        .arg(root.join("config"))
        .args([
            "--language",
            "en",
            "gui",
            "--smoke-test",
            "--smoke-test-input",
        ])
        .arg(&source)
        .arg("--smoke-test-output")
        .arg(&redacted)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("ALI native GUI process starts");

    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(status) = child.try_wait().expect("GUI process status is readable") {
            assert!(status.success(), "ALI GUI exited with {status}");
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("ALI GUI did not close after rendering its first frame");
        }
        thread::sleep(Duration::from_millis(25));
    }
    let candidate = std::fs::read_to_string(&redacted)
        .expect("the GUI scan creates a separate redacted candidate");
    assert!(candidate.contains("[REDACTED:email]"));
    assert!(!candidate.contains("client@example.com"));
}
