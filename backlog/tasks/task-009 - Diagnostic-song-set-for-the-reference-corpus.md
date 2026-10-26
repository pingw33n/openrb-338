---
id: TASK-009
title: Diagnostic song set for the reference corpus
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
labels:
  - tests
milestone: m-0
dependencies:
  - TASK-005
  - TASK-006
priority: high
type: task
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Build the diagnostic song set from docs/initial_investigation.md section 9, as JSON specs turned into .rbs by tools/rbs.py build (the specs and a script that builds all of them are committed; the songs per the storage decision): silence; single TB-303 notes (saw and square; low/high cutoff; resonance 0/64/127; accent; slide; both sound models); each TR-808 and TR-909 instrument alone (several knob values, accents, flam); each effect alone on one source (distortion incl. shape 0, PCF in both modes and several patterns, compressor on a channel and on master, delay straight and triplet with feedback 0/64/127); mixer level and pan sweeps through song automation; shuffle; loop start at bar 1 and bar 2; odd tempos (e.g. 97.3, 143.21 BPM) to exercise block quantization and the final partial chunk. Each song a few bars. A README in the song folder lists every song and what it isolates.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The JSON specs and a build script are committed; running it produces every song
- [ ] #2 Every song opens in the original without an error and plays what its README says (spot-checked by export for at least five songs)
- [ ] #3 The README lists each song and the engine feature it isolates
<!-- AC:END -->
