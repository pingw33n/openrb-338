//! The Python 3 interpreter the tests run `tools/*.py` with, shared by every crate's tests
//! (`#[path = "../../../tests/common/python.rs"] mod python;`).
//!
//! `ORB_PYTHON` (a program path, or a command split on whitespace such as `py -3`) if set, else the
//! first of `python3`, `python`, `py -3` on PATH that runs and is Python 3. Found once per test
//! binary. No interpreter is a panic naming `ORB_PYTHON`: CI has Python 3 on every runner, so a
//! test never skips for it.
#![allow(dead_code)]

use std::process::Command;
use std::sync::OnceLock;

/// The interpreter's program and leading arguments (`py -3`).
fn interpreter() -> &'static (String, Vec<String>) {
    static PY: OnceLock<(String, Vec<String>)> = OnceLock::new();
    PY.get_or_init(|| {
        let works = |prog: &str, pre: &[&str]| {
            Command::new(prog)
                .args(pre)
                .args([
                    "-I",
                    "-c",
                    "import sys; sys.exit(0 if sys.version_info[0] == 3 else 1)",
                ])
                .output()
                .is_ok_and(|o| o.status.success())
        };
        if let Some(p) = std::env::var_os("ORB_PYTHON").filter(|p| !p.is_empty()) {
            let p = p.to_string_lossy().into_owned();
            // The whole value is a program path (it may contain spaces), else split on
            // whitespace into a program and arguments (`py -3`).
            if works(&p, &[]) {
                return (p, Vec::new());
            }
            let mut words = p.split_whitespace();
            let prog = words.next().unwrap_or_default().to_string();
            let pre: Vec<String> = words.map(String::from).collect();
            let pre_ref: Vec<&str> = pre.iter().map(String::as_str).collect();
            assert!(
                works(&prog, &pre_ref),
                "ORB_PYTHON={p} does not run as Python 3"
            );
            return (prog, pre);
        }
        for (prog, pre) in [
            ("python3", &[][..]),
            ("python", &[][..]),
            ("py", &["-3"][..]),
        ] {
            if works(prog, pre) {
                return (prog.into(), pre.iter().map(|s| s.to_string()).collect());
            }
        }
        panic!(
            "no Python 3 found: tried python3, python and py -3 on PATH; \
             set ORB_PYTHON to a Python 3 interpreter"
        );
    })
}

/// A command running the interpreter in isolated mode (`-I`); add the script or `-c` and its
/// arguments.
pub fn python() -> Command {
    let (prog, pre) = interpreter();
    let mut c = Command::new(prog);
    c.args(pre).arg("-I");
    c
}
