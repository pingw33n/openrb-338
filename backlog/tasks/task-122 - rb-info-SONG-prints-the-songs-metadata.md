---
id: TASK-122
title: rb info SONG prints the song's metadata
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 14:22'
labels:
  - cli
dependencies: []
references:
  - crates/rb-cli/src/main.rs
  - crates/rb-ui/src/info.rs
  - docs/RBS42.txt
priority: medium
type: feature
ordinal: 142000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) for an `rb info <song>` command that prints metadata about a song. It is run on a temporary second lane next to task-108, and is not in a milestone.

What:
- `rb info SONG.rbs` (and a ReBirth 1.x MIDI-file song, as render and play accept) prints the song's metadata, the same facts File > Info shows in the UI (crates/rb-ui/src/info.rs, SongInfo::of):
  - the file;
  - the format and version (4.2, or 1.x);
  - the song info chunk: window title, the more text (0x0d shown as line breaks) and the web page;
  - the copyright;
  - the mod name, and whether it resolves to the built-in kit, to a mod in --mod-dir / RB_MOD_DIR (give the file), or is missing. Resolve it the way render does (mods::find_in_dir);
  - the tempo, the shuffle, the mode (pattern or song), the loop (start and length in bars), the synth sound (vintage or not), and the song length in bars if cheap.
- Share the code. Move the facts extraction out of rb-ui into a crate both can use (rb-cli's lib, or rbs if it needs only the song), and make rb-ui's SongInfo use it, so the two never disagree.
- Text output by default, one 'Label: value' per line, with multi-line text indented. Add `--json` for machine-readable output with stable keys; document the keys.
- Errors: an unreadable or invalid file exits 1 with the same messages as render. --mod-dir is optional; without it, mod resolution says 'not checked (no --mod-dir)', unless RB_MOD_DIR is set.
- Tests: crates/rb-cli/tests (info.rs), on committed corpus songs (tests/songs) and a 1.x song if one is committed:
  - the text form for one song;
  - the --json keys;
  - a missing file;
  - a mod resolved with RB_MOD_DIR (gated, skipped when it is absent).
- Document it in AGENTS.md (the commands list) and the CLI help.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 rb info SONG prints the song's metadata (format/version, title, text, web page, copyright, mod and its resolution, tempo, shuffle, mode, loop, synth sound) for 4.2 and 1.x songs; --json gives stable keys
- [ ] #2 The facts come from code shared with rb-ui's File > Info (no duplicate extraction); invalid files exit 1 with render's messages
- [ ] #3 Tests cover text, --json, an invalid file and gated mod resolution; AGENTS.md and the help describe it; cargo +nightly fmt --check, cargo test and clippy pass
<!-- AC:END -->
