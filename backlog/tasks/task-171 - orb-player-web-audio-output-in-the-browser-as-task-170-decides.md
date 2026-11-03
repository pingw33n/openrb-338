---
id: TASK-171
title: 'orb-player: web audio output in the browser, as task-170 decides'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-11-03 09:16'
labels:
  - engine
  - ui
milestone: m-6
dependencies:
  - TASK-169
  - TASK-170
references:
  - crates/orb-player/src/lib.rs
  - crates/orb-player/src/output.rs
  - crates/orb-player/tests
priority: high
type: feature
ordinal: 187000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the web version (m-6) plays songs live in the browser. orb-player is the live player under the panel (the engine in cpal's callback, rtrb queues for the UI's commands, a triple buffer of the engine's state back to the UI). task-170 is the user's decision on how this works on wasm32 (audio path A cpal webaudio / B threads + SharedArrayBuffer / C engine in an AudioWorklet with postMessage, and the sample rate); read its final summary first and build what it chose.

What to build:
- orb-player compiles for wasm32-unknown-unknown with the same public API the UI uses natively (Player, its commands, the state snapshot), the web output behind cfg(target_arch = "wasm32"); the native output (output.rs, cpal, rubato) is unchanged.
- The audio starts on the first user gesture (browsers keep an AudioContext suspended until one) and survives the tab being hidden and shown.
- The samples the engine produces reach the AudioContext unchanged at 44.1 kHz (x / 32768 as f32, as natively), unless task-170 decided otherwise.
- A minimal test page or wasm-bindgen-test that plays a corpus song with the built-in kit, used to check the output by ear and in the notes (dropouts under a busy main thread, latency of a knob change).

Not in scope: the UI's web build (its own task) and loading songs from the collections.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 orb-player builds for wasm32-unknown-unknown; the native build, its tests and clippy are unchanged
- [ ] #2 The web output follows task-170's decision (the decision named in the commit's Decision: line)
- [ ] #3 In Chrome and Firefox a corpus song plays at 44.1 kHz without dropouts while the main thread is idle, and the notes record how it behaves under a busy main thread and the latency of a command
- [ ] #4 Playback starts after a user gesture and resumes after the tab is hidden and shown again
- [ ] #5 docs/building.md (or a new docs/web.md) describes the web audio path
<!-- AC:END -->
