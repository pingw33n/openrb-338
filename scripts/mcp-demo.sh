#!/bin/sh
# A guided check of the `orb` MCP server: every tool called over the protocol, by a plain shell
# client, against the repository's own songs and references.
#
# It needs the server built (`sh scripts/local-env.sh cargo build --release -p orb-mcp`). The
# binary is taken from target/release, so what it checks is the working tree, not an installed
# copy.
#
# Usage:  sh scripts/mcp-demo.sh
#
# A tool that answers with a tool-level error prints the error; the script says which is
# expected and which is not.

set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
SERVER="$ROOT/target/release/orb-mcp"
CLIENT="$ROOT/crates/orb-mcp/tests/mcp_smoke.py"

if [ ! -x "$SERVER" ]; then
    echo "no server at $SERVER" >&2
    echo "build it: sh scripts/local-env.sh cargo build --release -p orb-mcp" >&2
    exit 1
fi

SONG=tests/songs/diag/songs/303_accent0.rbs
REF=tests/refs/diag/303_accent0.flac
OUT=${TMPDIR:-/tmp}/orb-mcp-demo.wav

# Every call names cwd, because the server is a separate process: its working directory is not
# this shell's. An absolute path needs no cwd.
run() {
    echo
    echo "=== $1 ==="
    shift
    printf '%s\n' "$*" | sed 's/^/    /'
    # The client exits 1 on a tool-level error, which is a result to show, not a failure of the
    # demo: `set -e` is lifted for the call.
    set +e
    python3 "$CLIENT" "$SERVER" "$@"
    code=$?
    set -e
    echo "    (client exit $code)"
}

echo "server: $SERVER"
echo "songs and references: $ROOT"

echo
echo "=== the tools the server serves ==="
# No tool name: the client's handshake ends with tools/list.
python3 "$CLIENT" "$SERVER"

run "orb_info: what the song says about itself" orb_info \
    "{\"song\":\"$SONG\",\"cwd\":\"$ROOT\"}"
run "orb_render: the loop as a WAV" orb_render \
    "{\"song\":\"$SONG\",\"output\":\"$OUT\",\"cwd\":\"$ROOT\"}"
run "orb_compare: the render against the reference" orb_compare \
    "{\"song\":\"$SONG\",\"reference\":\"$REF\",\"cwd\":\"$ROOT\"}"
run "orb_mod_list: the mods of the folder" orb_mod_list \
    "{\"mod_dir\":\"assets\",\"cwd\":\"$ROOT\"}"
run "a range of two bars" orb_render \
    "{\"song\":\"$SONG\",\"output\":\"$OUT\",\"range\":\"0:2\",\"cwd\":\"$ROOT\"}"

echo
echo "=== the WAV orb_render wrote ==="
if [ -f "$OUT" ]; then
    ls -l "$OUT"
    echo "sha256: $(sha256sum "$OUT" | cut -d' ' -f1)"
    echo
    echo "the reference's own sha256, from tests/refs/manifest.json:"
    python3 - "$ROOT" <<'PY'
import json, sys
root = sys.argv[1]
m = json.load(open(f"{root}/tests/refs/manifest.json"))
print("   ", m["diag"]["303_accent0"]["wav_sha256"])
PY
    echo "the two lines above are the same when the render matches the reference"
else
    echo "    (not written)"
fi

echo
echo "=== errors: what the client sees when a tool refuses ==="
run "a song that is not there" orb_info \
    "{\"song\":\"nope.rbs\",\"cwd\":\"$ROOT\"}"
run "an output extension that orb does not write" orb_render \
    "{\"song\":\"$SONG\",\"output\":\"out.mp3\",\"cwd\":\"$ROOT\"}"
run "a range whose end is before its start" orb_render \
    "{\"song\":\"$SONG\",\"output\":\"$OUT\",\"range\":\"4:0\",\"cwd\":\"$ROOT\"}"
