//! The crash hook (task-157): a panic on any thread writes a crash log, shows a native message
//! box and ends the process with a non-zero code, instead of the window vanishing. An app
//! started from Finder, the Start menu or a launcher has no visible stderr, and macOS writes no
//! crash report for a Rust panic.
//!
//! [`install`] replaces the panic hook at the start of `main`. The hook runs on the panicking
//! thread (the UI, the audio callback, the export thread) and:
//! 1. lets the default hook print the message and location to stderr;
//! 2. writes `crash-YYYYMMDD-HHMMSS.log` to [`log_dir`], keeping the newest [`KEEP`] logs;
//! 3. shows the box (unless `ORB_UI_NO_CRASH_DIALOG` is set) naming the log;
//! 4. exits with 101, the code of an unhandled panic.
//!
//! Every step ignores its own errors and panics (a panic inside a hook aborts the process, and
//! the stderr text is already out). A panic on another thread while the box is up waits for the
//! first one to end the process (it never returns into a half-dead program), and after 30 s
//! gives up and exits with 101 itself (it may be the main thread the box needs). The egui error box
//! cannot be used: the frame that panicked may be unusable.

use std::ffi::OsString;
use std::fmt::Write as _;
use std::io::Write as _;
use std::panic::{AssertUnwindSafe, PanicHookInfo};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::version::utc;

/// How many crash logs are kept (the newest).
pub const KEEP: usize = 20;

/// The exit code after a panic (what Rust's own unhandled panic gives).
pub const EXIT_CODE: i32 = 101;

/// The folder of the logs, from the environment: `ORB_UI_LOG_DIR` if set (tests), else
/// the platform's.
pub fn log_dir() -> Option<PathBuf> {
    platform_dir(std::env::consts::OS, |k| std::env::var_os(k))
}

/// [`log_dir`] for the system `os` (`std::env::consts::OS`) and an environment lookup:
/// - macOS: `~/Library/Logs/OpenRB-338`;
/// - Windows: `%LOCALAPPDATA%\OpenRB-338\logs`;
/// - others: `$XDG_STATE_HOME/OpenRB-338` or `~/.local/state/OpenRB-338`.
///
/// An empty variable counts as unset.
pub fn platform_dir(os: &str, env: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let get = |k: &str| env(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    if let Some(p) = get("ORB_UI_LOG_DIR") {
        return Some(p);
    }
    match os {
        "macos" => get("HOME").map(|h| h.join("Library/Logs/OpenRB-338")),
        "windows" => get("LOCALAPPDATA").map(|l| l.join("OpenRB-338").join("logs")),
        _ => get("XDG_STATE_HOME")
            .or_else(|| get("HOME").map(|h| h.join(".local/state")))
            .map(|s| s.join("OpenRB-338")),
    }
}

/// What the panic was, as the log and the box show it.
pub struct Panic<'a> {
    /// The version string of the About box.
    pub version: &'a str,
    /// Unix time, seconds.
    pub secs: i64,
    pub thread: &'a str,
    pub message: &'a str,
    /// `file:line:column`, when known.
    pub location: Option<&'a str>,
    pub backtrace: &'a str,
}

/// The log text.
pub fn report(p: &Panic) -> String {
    let [y, mo, d, h, mi, s] = utc(p.secs);
    let mut r = String::new();
    let _ = writeln!(r, "OpenRB-338 crash log");
    let _ = writeln!(
        r,
        "time:     {y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02} UTC"
    );
    let _ = writeln!(r, "version:  {}", p.version);
    let _ = writeln!(
        r,
        "system:   {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let _ = writeln!(r, "thread:   {}", p.thread);
    let _ = writeln!(r, "message:  {}", p.message);
    let _ = writeln!(r, "location: {}", p.location.unwrap_or("unknown"));
    let _ = writeln!(r, "\nbacktrace:\n{}", p.backtrace);
    r
}

/// The file name of the log of Unix time `secs`.
pub fn file_name(secs: i64) -> String {
    let [y, mo, d, h, mi, s] = utc(secs);
    format!("crash-{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}.log")
}

/// Writes `text` as the log of `secs` into `dir` (made if missing) and removes the oldest logs
/// beyond the newest `keep`. A second panic in the same second gets `-2`, `-3` ... before the
/// extension, so no log is overwritten. Returns the path.
pub fn write_log(dir: &Path, secs: i64, text: &str, keep: usize) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let name = file_name(secs);
    let stem = name.trim_end_matches(".log");
    // one past the highest sequence of this second still in the folder: a rotated-away "X.log"
    // must not be given out again, or the newest log would look the oldest
    let key = order_key(Path::new(&name)).0;
    let mut n = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| order_key(&e.path()))
        .filter(|(k, _)| *k == key)
        .map(|(_, s)| s + 1)
        .max()
        .unwrap_or(1);
    let (path, mut file) = loop {
        let candidate = if n == 1 {
            dir.join(&name)
        } else {
            dir.join(format!("{stem}-{n}.log"))
        };
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(f) => break (candidate, f),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && n < 1000 => n += 1,
            Err(e) => return Err(e),
        }
    };
    file.write_all(text.as_bytes())?;
    file.flush()?;
    drop(file);
    let _ = rotate(dir, keep);
    Ok(path)
}

/// Removes the oldest `crash-*.log` files of `dir` beyond the newest `keep` (the names sort by
/// time; other files stay).
pub fn rotate(dir: &Path, keep: usize) -> std::io::Result<()> {
    let mut logs: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("crash-") && n.ends_with(".log"))
        })
        .collect();
    // by (time, sequence): "X-2.log" is newer than "X.log", though it sorts before it as text
    logs.sort_by_key(|p| order_key(p));
    let extra = logs.len().saturating_sub(keep);
    for p in &logs[..extra] {
        let _ = std::fs::remove_file(p);
    }
    Ok(())
}

/// The sort key of a log's path: the time part of the name and the sequence (1 with no `-N`).
fn order_key(p: &Path) -> (String, u32) {
    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let stem = name.trim_start_matches("crash-").trim_end_matches(".log");
    // `YYYYMMDD-HHMMSS` or `YYYYMMDD-HHMMSS-N`
    let mut parts = stem.splitn(3, '-');
    let date = parts.next().unwrap_or("");
    let time = parts.next().unwrap_or("");
    let seq = parts.next().and_then(|n| n.parse().ok()).unwrap_or(1);
    (format!("{date}-{time}"), seq)
}

/// Writes a line to stderr; a failed write (a closed stderr) is ignored, because `eprintln!`
/// would panic and the box would be skipped.
fn say(line: &str) {
    let _ = writeln!(std::io::stderr(), "{line}");
}

/// How long a second panicking thread waits for the first to end the process.
const WAIT: Duration = Duration::from_secs(30);

/// The text of the box.
pub fn box_text(message: &str, location: Option<&str>, log: &Result<PathBuf, String>) -> String {
    let at = location.map(|l| format!(" ({l})")).unwrap_or_default();
    let saved = match log {
        Ok(p) => format!("A crash log was saved to {}.", p.display()),
        Err(e) => format!("The crash log could not be saved: {e}."),
    };
    format!(
        "OpenRB-338 stopped because of an internal error. {saved}\n\n{message}{at}\n\n\
         Unsaved changes were lost."
    )
}

/// Whether the box is turned off (`ORB_UI_NO_CRASH_DIALOG` set and not empty or `0`): for
/// tests, and for a machine with no desktop.
fn no_dialog() -> bool {
    std::env::var_os("ORB_UI_NO_CRASH_DIALOG").is_some_and(|v| !v.is_empty() && v != "0")
}

/// Set by the first panic that gets into the hook.
static HANDLING: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// This thread is inside the hook (a panic in the hook itself returns at once).
    static IN_HOOK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Installs the hook; `version` is the text of the About box.
pub fn install(version: &'static str) {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // stderr first: the default hook's text
        default(info);
        if IN_HOOK.with(|c| c.replace(true)) {
            return;
        }
        if HANDLING.swap(true, Ordering::SeqCst) {
            // another thread is writing the log and showing the box and will end the process.
            // This one waits, but not for ever: if it is the main thread, the box of a
            // thread may need it (macOS runs the box on the main queue).
            std::thread::sleep(WAIT);
            say("OpenRB-338: a second panic; the first one did not end the process in time");
            std::process::exit(EXIT_CODE);
        }
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| handle(version, info)));
        std::process::exit(EXIT_CODE);
    }));
}

/// The payload of a panic as text.
fn message(info: &PanicHookInfo) -> String {
    let p = info.payload();
    if let Some(s) = p.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = p.downcast_ref::<String>() {
        s.clone()
    } else {
        "(no message)".to_string()
    }
}

/// The log and the box for one panic.
fn handle(version: &str, info: &PanicHookInfo) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let thread = std::thread::current();
    let thread = thread.name().unwrap_or("<unnamed>");
    let message = message(info);
    let location = info.location().map(|l| l.to_string());
    let backtrace = std::backtrace::Backtrace::force_capture().to_string();
    let text = report(&Panic {
        version,
        secs,
        thread,
        message: &message,
        location: location.as_deref(),
        backtrace: &backtrace,
    });
    let log = match log_dir() {
        Some(dir) => write_log(&dir, secs, &text, KEEP).map_err(|e| e.to_string()),
        None => Err("no log folder".to_string()),
    };
    match &log {
        Ok(p) => say(&format!("OpenRB-338: crash log saved to {}", p.display())),
        Err(e) => say(&format!(
            "OpenRB-338: the crash log could not be saved: {e}"
        )),
    }
    if !no_dialog() {
        let _ = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("OpenRB-338")
            .set_description(box_text(&message, location.as_deref(), &log))
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(secs: i64) -> Panic<'static> {
        Panic {
            version: "2026.1030.0900 (abc1234)",
            secs,
            thread: "main",
            message: "missing art part switch_wave_127",
            location: Some("src/model.rs:10:5"),
            backtrace: "   0: frame",
        }
    }

    // 2026-10-30 09:15:07 UTC
    const T: i64 = 1_793_351_707;

    #[test]
    fn the_report_has_every_field() {
        let r = report(&p(T));
        for want in [
            "time:     2026-10-30 09:15:07 UTC",
            "version:  2026.1030.0900 (abc1234)",
            "thread:   main",
            "message:  missing art part switch_wave_127",
            "location: src/model.rs:10:5",
            "backtrace:\n   0: frame",
            std::env::consts::OS,
            std::env::consts::ARCH,
        ] {
            assert!(r.contains(want), "{want}\n{r}");
        }
    }

    #[test]
    fn the_file_is_named_by_the_utc_time() {
        assert_eq!(file_name(T), "crash-20261030-091507.log");
        assert_eq!(file_name(0), "crash-19700101-000000.log");
    }

    #[test]
    fn the_log_is_written_with_its_text_and_a_made_folder() {
        let t = tempfile::tempdir().unwrap();
        let dir = t.path().join("a/b");
        let path = write_log(&dir, T, "hello", KEEP).unwrap();
        assert_eq!(path, dir.join("crash-20261030-091507.log"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
        // the same second again: another file
        let path2 = write_log(&dir, T, "second", KEEP).unwrap();
        assert_eq!(path2, dir.join("crash-20261030-091507-2.log"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
        assert_eq!(std::fs::read_to_string(&path2).unwrap(), "second");
    }

    #[test]
    fn only_the_newest_20_logs_stay_and_other_files_are_left() {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(t.path().join("notes.txt"), "keep").unwrap();
        for i in 0..25 {
            write_log(t.path(), T + i, "x", KEEP).unwrap();
        }
        let mut names: Vec<String> = std::fs::read_dir(t.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        let logs: Vec<_> = names.iter().filter(|n| n.starts_with("crash-")).collect();
        assert_eq!(logs.len(), KEEP);
        assert_eq!(*logs[0], file_name(T + 5));
        assert_eq!(*logs[KEEP - 1], file_name(T + 24));
        assert!(names.contains(&"notes.txt".to_string()));
    }

    #[test]
    fn many_logs_in_one_second_keep_the_newest_20_by_sequence() {
        let t = tempfile::tempdir().unwrap();
        for i in 0..25 {
            write_log(t.path(), T, &format!("log {i}"), KEEP).unwrap();
        }
        let mut texts: Vec<String> = std::fs::read_dir(t.path())
            .unwrap()
            .map(|e| std::fs::read_to_string(e.unwrap().path()).unwrap())
            .collect();
        texts.sort();
        // the first 5 written are gone, the last 20 stay (-2 sorts before the plain name as text)
        let mut want: Vec<String> = (5..25).map(|i| format!("log {i}")).collect();
        want.sort();
        assert_eq!(texts, want);
        // and a log of the next second is newer than all of them
        write_log(t.path(), T + 1, "later", KEEP).unwrap();
        let texts: Vec<String> = std::fs::read_dir(t.path())
            .unwrap()
            .map(|e| std::fs::read_to_string(e.unwrap().path()).unwrap())
            .collect();
        assert_eq!(texts.len(), KEEP);
        assert!(texts.contains(&"later".to_string()));
        assert!(!texts.contains(&"log 5".to_string()));
    }

    #[test]
    fn the_folder_is_the_platforms() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |k: &str| {
                pairs
                    .iter()
                    .find(|(n, _)| *n == k)
                    .map(|(_, v)| OsString::from(v))
            }
        };
        let home = env(&[("HOME", "/h")]);
        assert_eq!(
            platform_dir("macos", home),
            Some("/h/Library/Logs/OpenRB-338".into())
        );
        let win = env(&[("LOCALAPPDATA", "C:\\L"), ("HOME", "/h")]);
        assert_eq!(
            platform_dir("windows", win),
            Some(PathBuf::from("C:\\L").join("OpenRB-338").join("logs"))
        );
        assert_eq!(
            platform_dir("linux", env(&[("HOME", "/h")])),
            Some("/h/.local/state/OpenRB-338".into())
        );
        assert_eq!(
            platform_dir("linux", env(&[("HOME", "/h"), ("XDG_STATE_HOME", "/x")])),
            Some("/x/OpenRB-338".into())
        );
        // empty counts as unset; the override wins everywhere
        assert_eq!(
            platform_dir("linux", env(&[("HOME", "/h"), ("XDG_STATE_HOME", "")])),
            Some("/h/.local/state/OpenRB-338".into())
        );
        assert_eq!(
            platform_dir("macos", env(&[("HOME", "/h"), ("ORB_UI_LOG_DIR", "/t")])),
            Some("/t".into())
        );
        assert_eq!(platform_dir("linux", env(&[])), None);
    }

    #[test]
    fn the_box_names_the_log_and_the_lost_work() {
        let t = box_text("boom", Some("a.rs:1:2"), &Ok("/l/crash.log".into()));
        assert!(t.starts_with("OpenRB-338 stopped because of an internal error."));
        assert!(t.contains("A crash log was saved to /l/crash.log."));
        assert!(t.contains("boom (a.rs:1:2)"));
        assert!(t.contains("Unsaved changes were lost."));
        let t = box_text("boom", None, &Err("denied".into()));
        assert!(t.contains("could not be saved: denied"));
    }
}
