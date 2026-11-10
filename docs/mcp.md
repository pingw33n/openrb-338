# orb-mcp: the OpenRB-338 tools over the Model Context Protocol

`crates/orb-mcp` is an MCP server over the same library the `orb` command line tool uses. An
agent that speaks MCP reads a song, renders it and compares the render with a reference without
a shell, and the answer is the `orb` answer: a tool wraps the function its `orb` command calls.

The transport is stdio: the server speaks JSON-RPC on stdin and stdout, so a client starts it as
a subprocess. Nothing is written to stdout outside the protocol; the warnings of a render (a song
in pattern mode, a faulting PCF call, a mod that is not the one the song names) go to stderr, as
`orb` prints them.

## The tools

| Tool | `orb` it is | Parameters |
|---|---|---|
| `orb_info` | `orb info SONG --json` | `song`, `mod_dir`, `mod_file`, `standard_kit`, `cwd` |
| `orb_render` | `orb render SONG -o OUT` | the song's, plus `output`, `range`, `idle_blocks`, `quality` |
| `orb_compare` | `orb compare SONG --reference REF` | the song's, plus `reference`, `idle_blocks` |
| `orb_mod_list` | the mod lookup of `orb render --mod-dir` (`mods::scan`) | `mod_dir`, `cwd` |
| `orb_song` | `tools/rbs.py json SONG` | `song`, `cwd` |
| `orb_build` | `tools/rbs.py build SPEC OUT` | `spec`, `output`, `cwd` |

`orb_song` and `orb_build` are how an agent composes: read a song as JSON, change what it needs,
write it back. The spec `orb_build` takes is exactly the model `orb_song` returns, merged over the
template song (`tests/songs/shipped/Silent Default Song.rbs`), so a spec names only what it
changes; `docs/beats.md` explains how to write one. Both run `tools/rbs.py` rather than
re-implementing the merge: two definitions of the same format would eventually disagree, and the
tool already has the byte-exact round trip. The interpreter is found as the tests find it —
`ORB_PYTHON` (a program or a command such as `py -3`), else `python3` on `PATH` — and the tool is
`tools/rbs.py` of the checkout, or `ORB_RBS_PY` for an installed binary.

A built file, a described model and an error message from `orb_build` are byte-for-byte what the
Python tool gives; the tests assert all three.

The song parameters are `orb render`'s: `song` is the `.rbs` (or ReBirth 1.x MIDI) file,
`mod_dir` the folder of recrypted mods (default `ORB_MOD_DIR`), `mod_file` a mod to use whatever
the song names, `standard_kit` the fall-back to the built-in kit. `orb_render`'s `output`
extension picks the encoding, as it does for `orb` (`.wav`, `.flac`, `.ogg`); `range` is
`START:END` in bars with the same limits (`orb-cli`'s `parse_range`); `idle_blocks` defaults to
9088, the reference corpus' capture protocol.

`cwd` is the one parameter `orb` has no equivalent of. A client starts the server as its own
process, so the server's working directory is not the client's: a relative `song`, `mod_dir`,
`mod_file`, `output` or `reference` would resolve against the wrong directory. `cwd` anchors them
to the directory the agent means (the repository root, usually). An absolute path is unaffected.
An agent that does not give `cwd` must give absolute paths.

Not a tool: `orb mod recrypt` and `orb mod pack-standard` read `RB20FUL.DAT`, `ReBirth.dll` and
`Rebirth.exe`, which are not in the repository and are not an agent's to handle. A mod that is not
recrypted fails with the `orb` message that says to run `orb mod recrypt`, and that is the whole
of what an agent needs to know about it.

## Exit codes and errors

`orb` reports through exit codes (0 success, 1 the song or the output failed, 2 a usage error).
A tool reports through the protocol: a failure is a tool-level error (`isError: true`) whose text
is the `orb` message, with its exit code appended (`... (exit code 1)`), so the client shows why
the song, the mod or the output was refused. A malformed range, an unknown output extension and a
quality out of range are refused before anything is written, as `orb` refuses them.

## Sessions

One server process serves one session. The renderer inside it keeps the tables the engine
computes once and the samples of every mod it loaded, so a second write of the same mod in a
session does not decode it again. A render holds a lock while it runs: two tool calls at once are
serialized (as two `orb render` processes would be, without computing the tables twice).

## Wiring it to a client

`.mcp.json` at the repository root registers the server as `orb`, started as `orb-mcp`. The
binary must be on the `PATH` (`cargo build --release -p orb-mcp`, then `target/release/orb-mcp`),
or the entry must give its path:

```json
{
  "mcpServers": {
    "orb": { "command": "/path/to/openrb-338/target/release/orb-mcp", "args": [] }
  }
}
```

The agent brief (`backlog/docs/doc-001`) is the place the worktree and testing rules live; an
agent with the `orb` server loaded reads a song with `orb_info` and checks an engine change with
`orb_compare`, instead of shelling out to `orb`.

## Testing

`crates/orb-mcp/tests/mcp.rs` starts the binary and speaks the protocol over stdio: the handshake,
`tools/list`, and a call of each tool, including the errors. The songs and the references are the
repository's own (`tests/songs/diag`, `tests/refs/diag`), so the tests need no external input and
run in a CI without the ReBirth CD image:

    cargo test -p orb-mcp

`crates/orb-mcp/tests/mcp_smoke.py` is the same conversation from a shell, for a quick look at a
build:

    python3 crates/orb-mcp/tests/mcp_smoke.py target/release/orb-mcp
    python3 crates/orb-mcp/tests/mcp_smoke.py target/release/orb-mcp orb_compare \
        '{"song":"tests/songs/diag/songs/303_accent0.rbs","reference":"tests/refs/diag/303_accent0.flac"}'

Checked on 2026-11-09 (Linux, rustc 1.99.1, `rmcp` 3.9.0): `cargo test -p orb-mcp` 6 pass,
`cargo clippy -p orb-mcp --all-targets -- -D warnings` clean, and the 145 `diag` songs of the
reference corpus compared through `orb_compare` (15 of them by hand through the Python smoke
client, all EQUAL; the set as a whole through `orb compare`, 153 of 153 with `shipped`).
