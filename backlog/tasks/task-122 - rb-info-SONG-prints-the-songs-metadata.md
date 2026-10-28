---
id: TASK-122
title: rb info SONG prints the song's metadata
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 14:22'
updated_date: '2026-10-28 14:48'
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
- [x] #1 rb info SONG prints the song's metadata (format/version, title, text, web page, copyright, mod and its resolution, tempo, shuffle, mode, loop, synth sound) for 4.2 and 1.x songs; --json gives stable keys
- [x] #2 The facts come from code shared with rb-ui's File > Info (no duplicate extraction); invalid files exit 1 with render's messages
- [x] #3 Tests cover text, --json, an invalid file and gated mod resolution; AGENTS.md and the help describe it; cargo +nightly fmt --check, cargo test and clippy pass
- [x] #4 rb info prints 'Last event: bar N' (1-based bar of the latest track event over all tracks, tick / 768; 'none' without events) and --json gives last_event_bar (null when none) and last_event_tick, from SongFacts for 4.2 and 1.x songs; the help explains that an .rbs song has no length field and the original runs to the end of bar 999 (tick 767231, 0x1001F484) unless the loop is on; tested on Calm (bar 176, checked with tools/rbs.py), an empty song and 1.x songs
- [x] #5 Control characters of the song's strings (ESC, C0, C1 0x80-0x9F, DEL) print as \xNN in the text form (newlines kept), tested with an ESC in the title
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. render.rs: split load_song into read_song (returns the parsed rbs::Song or Song1x plus the engine song) so rb info exits with render's exact messages. 2. New rb-cli/src/info.rs: SongFacts (of_source for a 4.2 rbs::Song, of_1x from the engine globals plus the 1.x block's song information at 0x3160), ModResolution + resolve_mod (find_in_dir; NotChecked without --mod-dir/RB_MOD_DIR), text and JSON renderers. 3. main.rs: 'rb info SONG [--mod-dir DIR] [--json]'. 4. rb-ui info.rs: SongInfo::of takes the song facts from SongFacts::of_source (no second extraction). 5. Tests crates/rb-cli/tests/info.rs: text, --json keys, missing file, invalid file, 1.x song built in the test, mod resolved via RB_MOD_DIR (gated). 6. AGENTS.md commands list + CLI help. Song length in bars is skipped (not cheap to define: the engine has no song-end notion beyond the loop); the loop is reported.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-accb3cfc3424eed48

Evidence. Shared code: crates/rb-cli/src/info.rs (SongFacts::of_source for 4.2, of_1x for 1.x, resolve_mod via mods::find_in_dir, to_text, to_json); render.rs read_song keeps the parsed file and load_song is a wrapper, so rb info's errors are render's messages (a test compares the stderr of info and render byte for byte); rb-ui SongInfo::of takes its facts from SongFacts::of_source. 1.x: globals from the engine song the import made; title/text/web page from the decoded sysex block at 0x3160 (docs/song_load.md 12.4); no copyright or mod addresses. Song length in bars is not printed (not cheap to define); the loop is. File > Info of a 1.x song still reads rb-ui's empty stand-in source: follow-up task created.

Real output (RB_MOD_DIR set to the recrypted mods): rb info tests/songs/shipped/Calm.rbs ->
File: tests/songs/shipped/Calm.rbs / Format: RBS 4.1 / Title: it down / Text: (3 indented lines) / Web page: http://www.bome.com / Copyright: (c)1997 Propellerhead Software, all rights reserved / Mod: Standard ReBirth / Mod resolution: built-in kit / Mod web: www.propellerheads.se / Mod ftp: ftp.propellerheads.se / Tempo: 130.000 BPM / Shuffle: 64 / Mode: song / Loop: on, start bar 0, 172 bars / Synth sound: ReBirth 2.0
rb info 'tests/songs/shipped/Alloy Cowboy Resurrection.rbs' ->
File: tests/songs/shipped/Alloy Cowboy Resurrection.rbs / Format: RBS 4.1 / Title: by bLANk / Text: (6 indented lines, from the quoted verse) / Web page: http://www.propellerheads.se/ / Copyright: (c)1997 Propellerhead Software, all rights reserved / Mod: Pitch Black Edition 2.0 / Mod resolution: file <RB_MOD_DIR>/PBE2.rbm / Mod web: http://www.propellerheads.se/ / Mod ftp: ftp://ftp.propellerheads.se/pub/rbmods/PBE2.rbm / Tempo: 152.000 BPM / Shuffle: 64 / Mode: song / Loop: on, start bar 0, 215 bars / Synth sound: ReBirth 2.0
rb info nope.rbs -> 'rb: error: nope.rbs: cannot read the song: No such file or directory (os error 2)', exit 1.
Corpus scan (rb info --json over 163 songs of tests/songs diag+shipped): 154 song/rebirth2, 7 song/vintage, 2 pattern/rebirth2; no failure.
Checks: cargo +nightly fmt --all --check ok; clippy --all-targets -D warnings ok; cargo test --release -q 549 pass; cargo test -p rb-cli -p rb-ui (debug) 241 pass, both with RB_MOD_DIR set.

Review round 1 (send back): task-123 set to priority low, milestone m-3, @agent-worker, and its description and AC #1 now name the format version and copyright (SongInfo::of reads the stand-in); SongFacts.version comment says e.g. 4.1; text mode escapes control characters (escape_controls in info.rs, applied in field()); test control_characters_of_the_song_are_escaped_in_the_text. Scope addition from the user: SongFacts.last_event_tick / last_event_bar(), 'Last event: bar N' / 'none', JSON last_event_bar and last_event_tick, help text on the missing length field. 4.2: max over the file's events of pos*24; 1.x: max tick of the import's engine tracks (the import always has the 909's tick-0 event, so a 1.x song is never 'none'). Calm: tick 134712 -> bar 176, same from tools/rbs.py (python: max(e.pos*24)//768+1). Re-run: fmt --check ok, clippy ok, release 551 pass, debug rb-cli+rb-ui 243 pass, RB_MOD_DIR set.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
rb info SONG [--mod-dir DIR] [--json] prints a song's metadata (format and version, title, text, web page, copyright, mod and its resolution, tempo, shuffle, mode, loop, synth sound, last event bar) for RBS 4.x and ReBirth 1.x songs. The facts come from rb_cli::info::SongFacts, which rb-ui's File > Info also uses (the UI shows no last event yet); errors come from render's loader (read_song). The text form escapes control characters as \xNN. Tests in crates/rb-cli/tests/info.rs cover the text, the --json keys, missing and invalid files (stderr equal to render's), a 1.x song built in the test, control-character escaping, the last event (Calm bar 176, empty song none, 1.x) and the mod resolution (not checked / missing / file, the last gated on RB_MOD_DIR). AGENTS.md and the help describe it, including that an .rbs song has no length field. Not done: File > Info of a 1.x song and the last event in the UI (task-123 and a later task). Branch worktree-agent-accb3cfc3424eed48, one commit. Tests: release 551 pass, debug rb-cli+rb-ui 243 pass.
<!-- SECTION:FINAL_SUMMARY:END -->
