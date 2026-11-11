//! The song's model as JSON, and writing a song from that model: `tools/rbs.py` over a subprocess.
//!
//! `rbs.py` owns the spec merge, the validation and the byte-exact round trip
//! (`docs/beats.md`), and `rbs::Song` parses what it writes. Re-implementing the merge in Rust
//! would give two definitions of the same format that could disagree, so this module runs the
//! Python tool and returns what it says, exactly as `orb-cli`'s own tools do with `tools/rbs.py`.
//!
//! The interpreter is found the way the tests find it (`AGENTS.md`): `ORB_PYTHON` (a program path,
//! or a command split on whitespace, such as `py -3`) if set, else `python3`, `python`, `py -3` on
//! PATH. No interpreter is an error naming `ORB_PYTHON`, not a silent failure.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{Fail, fail};

/// The repository's `tools/rbs.py`. The server is built from this checkout, so the tool sits two
/// directories above the crate; `ORB_RBS_PY` overrides it for an installed binary.
pub fn tool_path() -> PathBuf {
    if let Some(p) = std::env::var_os("ORB_RBS_PY").filter(|p| !p.is_empty()) {
        return PathBuf::from(p);
    }
    // crates/orb-mcp -> the repository root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(|root| root.join("tools").join("rbs.py"))
        .unwrap_or_else(|| PathBuf::from("tools/rbs.py"))
}

/// The interpreter and its leading arguments (`py -3`), as the tests find it.
fn interpreter() -> Result<(String, Vec<String>), Fail> {
    if let Some(p) = std::env::var_os("ORB_PYTHON").filter(|p| !p.is_empty()) {
        let p = p.to_string_lossy().into_owned();
        let mut words = p.split_whitespace();
        let program = words.next().unwrap_or_default().to_string();
        return Ok((program, words.map(str::to_string).collect()));
    }
    for candidate in ["python3", "python"] {
        if Command::new(candidate).arg("-c").arg("pass").output().is_ok() {
            return Ok((candidate.to_string(), Vec::new()));
        }
    }
    if Command::new("py").arg("-3").arg("-c").arg("pass").output().is_ok() {
        return Ok(("py".to_string(), vec!["-3".to_string()]));
    }
    Err(fail(
        1,
        "no Python 3 interpreter: set ORB_PYTHON to one, or put python3 on PATH",
    ))
}

/// Runs `rbs.py` with `args` and returns its stdout; a non-zero exit is the tool's stderr.
fn run(args: &[&str], stdin: Option<&[u8]>) -> Result<Vec<u8>, Fail> {
    let (program, leading) = interpreter()?;
    let tool = tool_path();
    if !tool.is_file() {
        return Err(fail(
            1,
            format!(
                "{}: no tools/rbs.py (set ORB_RBS_PY when orb-mcp is installed away from the \
                 repository)",
                tool.display()
            ),
        ));
    }
    let mut cmd = Command::new(&program);
    cmd.args(&leading).arg(&tool).args(args);
    let output = if let Some(input) = stdin {
        use std::io::Write;
        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        let mut child = cmd
            .spawn()
            .map_err(|e| fail(1, format!("cannot run {program}: {e}")))?;
        child
            .stdin
            .as_mut()
            .expect("piped stdin")
            .write_all(input)
            .map_err(|e| fail(1, format!("cannot write to {program}: {e}")))?;
        child
            .wait_with_output()
            .map_err(|e| fail(1, format!("{program} failed: {e}")))?
    } else {
        cmd.output()
            .map_err(|e| fail(1, format!("cannot run {program}: {e}")))?
    };
    if !output.status.success() {
        let msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(fail(
            output.status.code().unwrap_or(1) as u8,
            if msg.is_empty() {
                "tools/rbs.py failed".to_string()
            } else {
                msg
            },
        ));
    }
    Ok(output.stdout)
}

/// The model of `song` as JSON, the same text `rbs.py json SONG` prints.
pub fn describe(song: &Path) -> Result<String, Fail> {
    let out = run(&["json", &song.display().to_string()], None)?;
    String::from_utf8(out)
        .map_err(|e| fail(1, format!("tools/rbs.py printed no UTF-8: {e}")))
}

/// Writes `spec` (the JSON shape of [`describe`]) to `out`, the way `rbs.py build` does: merged
/// over the template song, validated, and written byte for byte.
pub fn build(spec: &str, out: &Path) -> Result<(), Fail> {
    // `rbs.py build SPEC OUT` reads SPEC from a file; the spec arrives here as text, so it goes to
    // a temporary file next to the output and is removed after the run.
    let dir = out.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let spec_path = dir.join(format!(
        ".orb-mcp-spec-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::write(&spec_path, spec)
        .map_err(|e| fail(1, format!("{}: cannot write: {e}", spec_path.display())))?;
    let result = run(
        &["build", &spec_path.display().to_string(), &out.display().to_string()],
        None,
    );
    let _ = std::fs::remove_file(&spec_path);
    result.map(|_| ())
}
