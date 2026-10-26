---
id: TASK-028
title: 'rb CLI: render .rbs to FLAC, Ogg Vorbis and WAV'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
labels:
  - cli
milestone: m-1
dependencies:
  - TASK-021
  - TASK-020
priority: high
type: feature
ordinal: 28000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The `rb` command: `rb render IN.rbs -o OUT.flac|OUT.ogg|OUT.wav` renders the song loop as the original export does (44.1 kHz 16-bit stereo) and encodes by the output extension: FLAC lossless (a pure-Rust encoder preferred), Ogg Vorbis with `--quality Q` (a maintained Rust crate; a C library binding is acceptable if no pure-Rust encoder is usable, say which in the notes), WAV for comparison. Options for the built-in samples' source (per the decision), `--range START:END` in bars to override the loop, and the initial-state preset of the capture protocol. Clear errors for unreadable songs. Integration tests: render a corpus song to each format; decode the FLAC and compare it sample-exact with the WAV.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 rb render writes .flac, .ogg and .wav, chosen by extension, with the options described
- [ ] #2 An integration test decodes the FLAC output and finds it sample-identical to the WAV output
- [ ] #3 The encoder crates and why they were chosen are in the task's notes
<!-- AC:END -->
