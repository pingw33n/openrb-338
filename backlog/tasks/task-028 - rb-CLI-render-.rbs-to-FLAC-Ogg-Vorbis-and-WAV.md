---
id: TASK-028
title: 'rb CLI: render .rbs to FLAC, Ogg Vorbis and WAV'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 19:52'
labels:
  - cli
milestone: m-1
dependencies:
  - TASK-021
  - TASK-023
priority: high
type: feature
ordinal: 23500
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The `rb` command: `rb render IN.rbs -o OUT.flac|OUT.ogg|OUT.wav` renders the song loop as the original export does (44.1 kHz 16-bit stereo) and encodes by the output extension: FLAC lossless (a pure-Rust encoder preferred), Ogg Vorbis with `--quality Q` (a maintained Rust crate; a C library binding is acceptable if no pure-Rust encoder is usable, say which in the notes), WAV for comparison. Options for the built-in samples' source (per the decision), `--range START:END` in bars to override the loop, and the initial-state preset of the capture protocol. Clear errors for unreadable songs. Integration tests: render a corpus song to each format; decode the FLAC and compare it sample-exact with the WAV.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 rb render writes .flac, .ogg and .wav, chosen by extension, with the options described
- [x] #2 An integration test decodes the FLAC output and finds it sample-identical to the WAV output
- [x] #3 The encoder crates and why they were chosen are in the task's notes
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
clap CLI with a render subcommand; encode.rs (WAV via the engine's wav_bytes, flacenc, vorbis_rs); warnings for unimplemented devices and mods; integration tests in crates/rb-cli/tests/render.rs.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-26, per the user: run this task once there is something to listen to, right after the TB-303 devices (task-023) and before the drums and effects (task-024..027).

Supervisor 2026-10-26, per the user: no longer waits for the sample loader (task-020); the CLI renders 303 songs first, and task-020 adds the --rebirth-dll sample source to it.

Branch: worktree-agent-ad52468e32d97ad4e

Encoders: FLAC = flacenc 0.5.1 (pure Rust, default config, fixed block size; builds on macOS with the pinned toolchain; claxon decodes it sample-identical in the test). Ogg Vorbis = vorbis_rs 0.5.6, bindings to the aoTuV libvorbis (C): no usable pure-Rust Vorbis encoder exists; it builds on macOS with the bundled C sources; --quality maps to QualityVbr target_quality (-0.2..1.0, default 0.5). WAV = rb_engine::frame::export::wav_bytes, byte-identical to the engine export (silence test: sha256 equals tests/refs/manifest.json diag.silence.wav_sha256 at the default 9088 idle blocks). CLI: clap derive. Exit codes: 0 ok, 1 unreadable/invalid song or unwritable output, 2 usage (extension, range, quality). --range START:END = 0-based bars (tick = bar*768 as GLOB loop fields), END exclusive, overrides loop_start/loop_end. --idle-blocks N default 9088. Warnings to stderr (not failures): mod other than Standard ReBirth; enabled TR-808/909 with any step, enabled distortion/delay/PCF/compressor. Drums 'has notes' = any nonzero step in any of the 32 patterns, so it can over-warn. Not done: --rebirth-dll sample source (task-020). Listening renders (FLAC and Ogg, 303-only songs): scratchpad listen/Trakka Rekka.{flac,ogg} and ReBirth 1.0 Default Song.{flac,ogg}. Tests: cargo test --release all pass (rb-cli 4 new), clippy -D warnings clean.

Review round 1: (1) range test now exact: 0:1 gives 88199 frames, not 88200 (the export's chunk arithmetic, ftol truncation in export::chunk_sizes, drops one frame; engine behavior, not changed), 0:2 gives 176400. (2) pattern-mode songs (song_mode 0) now warn that the original cannot export them and the output is the engine's song-mode render; test pattern_mode_warns on Silent Default Song. (3) drum warning now needs the drum enabled and a pattern with a step among params[1] plus controller-1 events of tracks 3/4; no event means the initial selection plays throughout. (4) the sample-source option is deferred to task-020. Listening files unchanged (output not affected). cargo test --release, clippy, fmt pass.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
rb render IN.rbs -o OUT.{wav,flac,ogg} with --quality, --range, --idle-blocks, warnings (mod, pattern mode, unimplemented devices) and exit codes. The sample-source option (--rebirth-dll) is deferred to task-020. Encoders: flacenc (pure Rust), vorbis_rs (libvorbis), WAV from the engine. Tests: 5 integration tests (formats agree, FLAC == WAV sample-exact, silence WAV sha256 == manifest, exact range lengths, pattern-mode warning, error exits); cargo test --release and clippy pass. Branch worktree-agent-ad52468e32d97ad4e, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->
