//! The crash hook end to end (task-157): `OpenRB-338 --debug-panic` panics on purpose before any
//! window, with the box off (`ORB_UI_NO_CRASH_DIALOG`) and the log folder a temp folder
//! (`ORB_UI_LOG_DIR`, so the user's real log folder is never written). The process exits with
//! 101, stderr has the panic text, and one `crash-*.log` appears with the fields.

use std::process::Command;

fn run(log_dir: &std::path::Path, home: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_OpenRB-338"))
        .arg("--debug-panic")
        .env("ORB_UI_LOG_DIR", log_dir)
        .env("ORB_UI_NO_CRASH_DIALOG", "1")
        // nothing of the user's: a settings file and a home that are temp
        .env("ORB_UI_CONFIG", home.join("ui.toml"))
        .env("HOME", home)
        .output()
        .unwrap()
}

#[test]
fn a_deliberate_panic_exits_101_and_writes_the_log() {
    let t = tempfile::tempdir().unwrap();
    let logs = t.path().join("logs");
    let o = run(&logs, t.path());
    assert_eq!(o.status.code(), Some(101), "{o:?}");
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("--debug-panic: a deliberate panic"), "{err}");
    assert!(err.contains("crash log saved to"), "{err}");

    let files: Vec<_> = std::fs::read_dir(&logs)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1, "{files:?}");
    let name = files[0].file_name().unwrap().to_str().unwrap();
    assert!(
        name.starts_with("crash-") && name.ends_with(".log"),
        "{name}"
    );
    let log = std::fs::read_to_string(&files[0]).unwrap();
    for want in [
        "OpenRB-338 crash log",
        "UTC",
        "version:  ",
        "thread:   main",
        "message:  --debug-panic: a deliberate panic",
        "main.rs:",
        "backtrace:",
        std::env::consts::OS,
        std::env::consts::ARCH,
    ] {
        assert!(log.contains(want), "{want}\n{log}");
    }
    // a backtrace with a frame of the hook, though RUST_BACKTRACE is not set
    let bt = log.split_once("\nbacktrace:\n").unwrap().1;
    assert!(bt.contains("orb_ui::crash"), "{log}");
}

#[test]
fn a_second_crash_adds_a_second_log() {
    let t = tempfile::tempdir().unwrap();
    let logs = t.path().join("logs");
    for _ in 0..2 {
        assert_eq!(run(&logs, t.path()).status.code(), Some(101));
    }
    assert_eq!(std::fs::read_dir(&logs).unwrap().count(), 2);
}
