//! The MCP server end to end: a client asks for the tools and calls them over stdio, the
//! transport an MCP client uses, so these tests cover the protocol wiring, not only the code
//! behind a tool.
//!
//! The songs and the references are the repository's own (`tests/songs/diag`, `tests/refs/diag`),
//! so no external input is needed: these tests run everywhere, including a CI without the
//! ReBirth CD image.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

/// The repository root: this test binary lives in `target/<profile>/deps`.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the repository root")
        .to_path_buf()
}

/// A server process with its protocol lines read from stdout.
struct Server {
    child: std::process::Child,
    out: BufReader<std::process::ChildStdout>,
    id: u64,
}

impl Server {
    fn start() -> Server {
        let mut child = Command::new(env!("CARGO_BIN_EXE_orb-mcp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the server starts");
        let out = BufReader::new(child.stdout.take().expect("stdout"));
        Server { child, out, id: 0 }
    }

    /// One request, waiting for the response with its id. A message that is not a response (a
    /// notification or a server request) is skipped.
    fn request(&mut self, method: &str, params: Value) -> Value {
        self.id += 1;
        let msg = json!({"jsonrpc": "2.0", "id": self.id, "method": method, "params": params});
        let stdin = self.child.stdin.as_mut().expect("stdin");
        writeln!(stdin, "{msg}").expect("the request is written");
        stdin.flush().expect("the request is flushed");
        loop {
            let mut line = String::new();
            assert!(
                self.out.read_line(&mut line).expect("a reply line") > 0,
                "the server closed the connection"
            );
            let reply: Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if reply.get("id") == Some(&json!(self.id)) {
                return reply;
            }
        }
    }

    fn notify(&mut self, method: &str, params: Value) {
        let msg = json!({"jsonrpc": "2.0", "method": method, "params": params});
        let stdin = self.child.stdin.as_mut().expect("stdin");
        writeln!(stdin, "{msg}").expect("the notification is written");
        stdin.flush().expect("the notification is flushed");
    }

    /// The handshake every client makes, then `tools/call`'s result.
    fn call(&mut self, tool: &str, args: Value) -> Value {
        self.request(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "orb-mcp-test", "version": "0"},
            }),
        );
        self.notify("notifications/initialized", json!({}));
        self.request("tools/call", json!({"name": tool, "arguments": args}))["result"].clone()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The text of a tool result.
fn text(result: &Value) -> String {
    result["content"]
        .as_array()
        .expect("a content array")
        .iter()
        .map(|b| b["text"].as_str().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether a tool result is a tool-level error.
fn is_error(result: &Value) -> bool {
    result["isError"].as_bool().unwrap_or(false)
}

#[test]
fn initialize_reports_the_server_and_lists_the_tools() {
    let mut s = Server::start();
    let init = s.request(
        "initialize",
        json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "orb-mcp-test", "version": "0"},
        }),
    );
    assert_eq!(init["result"]["serverInfo"]["name"], "orb-mcp");
    s.notify("notifications/initialized", json!({}));
    let tools = s.request("tools/list", json!({}));
    let list = tools["result"]["tools"].as_array().expect("the tool list");
    let names: Vec<&str> = list
        .iter()
        .map(|t| t["name"].as_str().expect("a tool name"))
        .collect();
    // The router's order is not the protocol's: the set is what the server serves.
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(
        sorted,
        [
            "orb_build",
            "orb_compare",
            "orb_info",
            "orb_mod_list",
            "orb_render",
            "orb_song",
        ],
        "the tools the server serves"
    );
    for t in list {
        let name = t["name"].as_str().expect("a tool name");
        assert!(
            t["description"].as_str().is_some_and(|d| !d.is_empty()),
            "{name} has a description"
        );
        // The parameter that names the work each tool does.
        let key = match name {
            "orb_mod_list" => "mod_dir",
            "orb_build" => "spec",
            _ => "song",
        };
        assert!(
            t["inputSchema"]["properties"].get(key).is_some(),
            "{name} takes {key}: {}",
            t["inputSchema"]["properties"]
        );
    }
}

#[test]
fn info_prints_the_json_of_orb_info() {
    let song = root().join("tests/songs/shipped/Silent Default Song.rbs");
    let mut s = Server::start();
    let result = s.call("orb_info", json!({"song": song.display().to_string()}));
    assert!(!is_error(&result), "{}", text(&result));
    let facts: Value = serde_json::from_str(&text(&result)).expect("the JSON of orb info");
    assert_eq!(facts["format"], "rbs");
    assert_eq!(facts["mod_name"], "Standard ReBirth");
    assert_eq!(facts["mod_resolution"], "builtin");
}


#[test]
fn render_writes_the_wav_and_compare_finds_it_equal() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let out = dir.path().join("out.wav");
    let song = root().join("tests/songs/diag/songs/303_accent0.rbs");
    let reference = root().join("tests/refs/diag/303_accent0.flac");
    let mut s = Server::start();

    let rendered = s.call(
        "orb_render",
        json!({"song": song.display().to_string(), "output": out.display().to_string()}),
    );
    assert!(!is_error(&rendered), "{}", text(&rendered));
    let bytes = std::fs::metadata(&out).expect("the output exists").len();
    // 44 bytes of header and 4 per stereo frame: the rest is a whole number of frames.
    assert!(bytes > 44, "{bytes} bytes written");
    assert_eq!(bytes % 4, 44 % 4, "a whole number of frames");

    let compared = s.call(
        "orb_compare",
        json!({
            "song": song.display().to_string(),
            "reference": reference.display().to_string(),
        }),
    );
    assert!(!is_error(&compared), "{}", text(&compared));
    assert!(text(&compared).contains("EQUAL"), "{}", text(&compared));
}

#[test]
fn a_song_that_cannot_be_read_is_a_tool_error() {
    let mut s = Server::start();
    let result = s.call("orb_info", json!({"song": "/nonexistent/nope.rbs"}));
    assert!(is_error(&result), "a missing song is an error");
    let msg = text(&result);
    assert!(msg.contains("cannot read the song"), "{msg}");
    assert!(msg.contains("exit code 1"), "{msg}");

    let result = s.call(
        "orb_render",
        json!({"song": "/nonexistent/nope.rbs", "output": "/tmp/orb-mcp-test.mp3"}),
    );
    assert!(is_error(&result));
    assert!(
        text(&result).contains("must be .wav, .flac or .ogg"),
        "{}",
        text(&result)
    );
}

#[test]
fn a_bad_range_is_refused_as_orb_refuses_it() {
    let song = root().join("tests/songs/diag/songs/303_accent0.rbs");
    let dir = tempfile::tempdir().expect("a temporary directory");
    let out = dir.path().join("out.wav");
    let mut s = Server::start();
    for (range, want) in [
        ("4:0", "START must be below END"),
        ("0", "expected START:END"),
        ("0:99999999", "END is past bar"),
    ] {
        let result = s.call(
            "orb_render",
            json!({
                "song": song.display().to_string(),
                "output": out.display().to_string(),
                "range": range,
            }),
        );
        assert!(is_error(&result), "{range} is refused");
        assert!(text(&result).contains(want), "{range}: {}", text(&result));
    }
}

#[test]
fn cwd_anchors_the_relative_paths() {
    // A client starts the server as its own process, so a relative path of a tool counts from
    // `cwd` when one is given, not from the directory the server happened to start in.
    let root = root();
    let mut s = Server::start();
    let result = s.call(
        "orb_info",
        json!({
            "song": "tests/songs/diag/songs/303_accent0.rbs",
            "cwd": root.display().to_string(),
        }),
    );
    assert!(!is_error(&result), "{}", text(&result));
    let facts: Value = serde_json::from_str(&text(&result)).expect("the JSON of orb info");
    assert_eq!(facts["tempo_bpm"], 120.0);

    // Without `cwd` the same relative path is not found from a directory that is not the root:
    // the error names the path as given.
    let result = s.call(
        "orb_mod_list",
        json!({"mod_dir": "assets", "cwd": root.display().to_string()}),
    );
    assert!(!is_error(&result), "{}", text(&result));
    assert!(text(&result).contains("Standard ReBirth"), "{}", text(&result));
}

#[test]
fn mod_list_names_the_mods_of_a_folder() {
    let mut s = Server::start();
    let dir = root().join("assets");
    let result = s.call("orb_mod_list", json!({"mod_dir": dir.display().to_string()}));
    assert!(!is_error(&result), "{}", text(&result));
    let msg = text(&result);
    assert!(msg.contains("Standard ReBirth"), "{msg}");
    assert!(msg.contains("Standard.rbm"), "{msg}");
}

/// The Python tool the two song tools wrap, as the tests find the interpreter: the same repository
/// checkout (crates/orb-mcp -> the root) and `rbs.py` next to the songs.
fn rbspy() -> std::path::PathBuf {
    root().join("tools").join("rbs.py")
}

/// Runs `tools/rbs.py` with `args` and returns its stdout, failing the test on a bad exit.
fn python(args: &[&str]) -> Vec<u8> {
    let program = std::env::var("ORB_PYTHON").unwrap_or_else(|_| "python3".to_string());
    let out = std::process::Command::new(program)
        .arg(rbspy())
        .args(args)
        .output()
        .expect("run tools/rbs.py");
    assert!(
        out.status.success(),
        "tools/rbs.py {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

#[test]
fn song_returns_the_model_tools_rbs_py_returns() {
    let song = root().join("tests/songs/diag/songs/303_accent0.rbs");
    let mut s = Server::start();
    let result = s.call("orb_song", json!({"song": song.display().to_string()}));
    assert!(!is_error(&result), "{}", text(&result));
    let from_mcp: Value = serde_json::from_str(&text(&result)).expect("the model as JSON");
    let expected: Value =
        serde_json::from_slice(&python(&["json", &song.display().to_string()]))
            .expect("the model tools/rbs.py prints");
    assert_eq!(from_mcp, expected, "orb_song and rbs.py json disagree");
}

#[test]
fn build_writes_the_same_file_as_tools_rbs_py() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let spec = json!({
        "usri": {"window_title": "Via MCP"},
        "glob": {"tempo": 100000, "song_mode": 1, "loop": 1, "loop_end": 3072},
        "tr909": {"patterns": {"0": {"length": 16, "steps": {
            "0": [0,1,0,0,0,0,0,0,0,0,0,0],
            "8": [0,1,0,0,0,0,0,0,0,0,0,0]
        }}}},
        "tracks": {"tr909": [{"pos": 0, "ctrl": "pattern", "value": 0}]}
    });
    let out = dir.path().join("build.rbs");

    // Through the server.
    let mut s = Server::start();
    let built = s.call(
        "orb_build",
        json!({"spec": spec, "output": out.display().to_string()}),
    );
    assert!(!is_error(&built), "{}", text(&built));
    let through_mcp = std::fs::read(&out).expect("the file orb_build wrote");

    // Through the tool, from the same spec.
    let spec_path = dir.path().join("spec.json");
    std::fs::write(&spec_path, serde_json::to_vec(&spec).unwrap()).expect("write the spec");
    let expected_path = dir.path().join("python.rbs");
    python(&[
        "build",
        &spec_path.display().to_string(),
        &expected_path.display().to_string(),
    ]);
    let through_python = std::fs::read(&expected_path).expect("the file rbs.py wrote");

    assert_eq!(
        through_mcp, through_python,
        "orb_build and rbs.py build disagree"
    );
}

#[test]
fn a_spec_the_tool_rejects_is_the_same_tool_error() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let mut s = Server::start();
    // An unknown field: rbs.py's validate refuses it, and the server passes the message on.
    let result = s.call(
        "orb_build",
        json!({
            "spec": {"tr909": {"params": {"no_such_knob": 1}}},
            "output": dir.path().join("bad.rbs").display().to_string(),
        }),
    );
    assert!(is_error(&result), "an unknown field is an error");
    let msg = text(&result);
    assert!(
        msg.contains("unknown field tr909.params.no_such_knob"),
        "{msg}"
    );
    // And the same message comes out of the tool directly.
    let spec_path = dir.path().join("bad.json");
    std::fs::write(
        &spec_path,
        br#"{"tr909": {"params": {"no_such_knob": 1}}}"#,
    )
    .expect("write the spec");
    let program = std::env::var("ORB_PYTHON").unwrap_or_else(|_| "python3".to_string());
    let out = std::process::Command::new(program)
        .arg(rbspy())
        .args([
            "build",
            &spec_path.display().to_string(),
            &dir.path().join("bad2.rbs").display().to_string(),
        ])
        .output()
        .expect("run tools/rbs.py");
    assert!(!out.status.success(), "rbs.py rejects the spec too");
    let from_python = String::from_utf8_lossy(&out.stderr);
    assert!(
        from_python.contains("unknown field tr909.params.no_such_knob"),
        "{from_python}"
    );
}


