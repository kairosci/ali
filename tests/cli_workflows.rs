use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

#[test]
fn cli_redacts_a_file_and_creates_a_bounded_handoff() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config = root.join("config");
    let temporary = tempfile::tempdir().expect("temporary test directory is available");
    let source = temporary.path().join("private.txt");
    let redacted = temporary.path().join("private.redacted.txt");
    fs::write(&source, "Contact the client at client@example.com.\n")
        .expect("synthetic source is written");

    let scan = Command::new(env!("CARGO_BIN_EXE_ali"))
        .args(["--config-dir", config.to_str().unwrap(), "privacy", "scan"])
        .arg("--input")
        .arg(&source)
        .arg("--output")
        .arg(&redacted)
        .output()
        .expect("ALI privacy scan process starts");
    assert!(
        scan.status.success(),
        "{}",
        String::from_utf8_lossy(&scan.stderr)
    );
    let candidate = fs::read_to_string(&redacted).expect("ALI writes a separate candidate");
    assert!(candidate.contains("[REDACTED:email]"));
    assert!(!candidate.contains("client@example.com"));

    let handoff = temporary.path().join("handoff.md");
    let input = br#"{"session_id":"test-session","failed_agent":"codex-cli","failure":"crash","verified_progress":["Created a local redacted candidate."],"next_action":"Contact client@example.com only after review."}"#;
    let mut child = Command::new(env!("CARGO_BIN_EXE_ali"))
        .args([
            "--config-dir",
            config.to_str().unwrap(),
            "handoff",
            "create",
            "--output",
        ])
        .arg(&handoff)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("ALI handoff process starts");
    child
        .stdin
        .take()
        .expect("handoff stdin is piped")
        .write_all(input)
        .expect("synthetic handoff input is sent");
    let result = child.wait_with_output().expect("ALI handoff completes");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let document = fs::read_to_string(&handoff).expect("ALI creates the handoff file");
    assert!(document.lines().count() <= 50);
    assert!(!document.contains("client@example.com"));
}
