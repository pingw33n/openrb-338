---
id: TASK-176
title: 'orb-mcp: orb_build writes a song from a JSON spec, and orb_song describes one'
status: Done
assignee: []
created_date: '2026-11-10T13:37:00Z'
updated_date: '2026-11-10T13:42:00Z'
labels:
  - cli
dependencies: []
priority: high
type: feature
ordinal: 192000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The MCP server (crates/orb-mcp, docs/mcp.md) reads a song, renders it, compares a render with a reference and lists mods. It cannot WRITE a song, so an agent that wants to compose has to leave the protocol and run tools/rbs.py build from a shell, which is what every piece written in this fork did.

This closes the loop: a tool that takes the same spec rbs.py build takes (the JSON shape of rbs.py json), merges it over the template, validates it and writes the .rbs; and a tool that returns a song as that JSON, so an agent can read, alter and write back.

The merge, the validation and the byte-exact round trip already exist in tools/rbs.py. The work is to expose the same operation over MCP through orb-cli, not to reimplement the format.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 a spec built through orb_build and through `python3 tools/rbs.py build` gives byte-identical .rbs files
- [x] #2 orb_song of a corpus song equals `python3 tools/rbs.py json` of it
- [x] #3 a spec rbs.py rejects (unknown field, unsorted track, pattern change off a bar) is a tool-level error with the same message
- [x] #4 crates/orb-mcp/tests/mcp.rs covers both tools over stdio, with no external input
- [x] #5 docs/mcp.md documents the two tools and the spec shape, and points at docs/beats.md
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: feat/orb-build, from main.

Approach: run tools/rbs.py over a subprocess (src/song.rs) rather than re-implementing the merge in Rust. The tool already owns the merge, the validation and the byte-exact round trip; a second definition in Rust would be a second thing that could disagree. The interpreter is found as the tests find it (ORB_PYTHON, else python3 on PATH), and the tool path is the checkout's tools/rbs.py, with ORB_RBS_PY for an installed binary.

Evidence:
- orb_build of a spec and `python3 tools/rbs.py build` of the same spec give the same bytes: cmp identical, 16754 bytes.
- orb_song of tests/songs/diag/songs/303_accent0.rbs equals `rbs.py json` of it, as parsed JSON.
- an unknown field gives the same message through both paths: unknown field tr909.params.no_such_knob
- cargo test -p orb-mcp: 10 pass, 3 of them new. cargo clippy --all-targets -- -D warnings: nothing.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
orb_build and orb_song close the loop: an agent can read a song, change it and write it back over MCP, without a shell.

Both wrap tools/rbs.py over a subprocess (crates/orb-mcp/src/song.rs) instead of duplicating the merge in Rust, so a built file, a described model and a rejected spec are byte-for-byte the Python tool's answers. Verified: identical .rbs (cmp), equal models, the same error text.

Branch feat/orb-build, commits 1. Tests: cargo test -p orb-mcp 10 pass; clippy clean.
<!-- SECTION:FINAL_SUMMARY:END -->
