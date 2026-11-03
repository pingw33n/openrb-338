---
id: TASK-170
title: 'Decide: the web version''s audio path, threading and export formats'
status: To Do
assignee:
  - '@pingw33n'
created_date: '2026-11-03 09:16'
labels:
  - decision
  - ui
milestone: m-6
dependencies: []
references:
  - crates/orb-player/src/lib.rs
  - crates/orb-player/src/output.rs
  - crates/orb-ui/src/menu.rs
  - crates/orb-cli/Cargo.toml
priority: high
type: spike
ordinal: 186000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the web version (m-6) needs audio output in the browser, and the native design does not carry over as is. Natively, orb-player runs the engine in cpal's audio callback on its own thread, talks to the UI through lock-free queues (rtrb) and a triple buffer of the engine's state (the LEDs, the step position), and the UI exports on a std::thread. In the browser: wasm32-unknown-unknown has no threads unless it is built with atomics and run cross-origin isolated (COOP/COEP headers), which GitHub Pages cannot set (the workaround is a service worker that adds them, e.g. coi-serviceworker, which reloads the page once and breaks some embedding); the audio callback is either on the main thread or in an AudioWorklet (its own JS realm).

Options for the audio path:
A. cpal's webaudio backend as is (the `wasm-bindgen` feature): the engine runs on the main thread and its buffers are scheduled ahead as AudioBufferSourceNodes. Least work, orb-player keeps its shape; but audio drops out whenever the UI frame or the tab is busy, and the scheduling ahead adds latency to knob and pattern changes.
B. wasm threads + SharedArrayBuffer: the engine in an AudioWorklet sharing memory with the UI, the native design (rtrb, triple buffer) nearly unchanged. Needs nightly-only build-std with atomics, and cross-origin isolation via the service worker workaround on Pages.
C. The engine in an AudioWorklet as its own wasm instance (orb-engine only, a small module), the UI talking to it with postMessage (commands one way, state snapshots back), no shared memory. Glitch-free audio independent of the UI, stable toolchain, no isolation headers; costs a message layer in orb-player for the web (the native path stays as is) and a few ms of state-display lag.

Recommendation: C.

Also to settle:
- Export on the web: WAV and FLAC are pure Rust (flacenc); Ogg uses vorbis_rs (libvorbis, C), which does not build for wasm32-unknown-unknown without an emscripten toolchain. Recommendation: WAV and FLAC on the web, Ogg hidden; the export runs on the main thread in slices per frame (no thread) with the progress shown, then the browser downloads the file.
- Settings (recent files, preferences) in localStorage instead of the settings file; the crash log becomes a console message plus the error box. Recommendation: yes.
- The 44.1 kHz rate: an AudioContext can be created at 44100 Hz in every current browser, so no resampling is needed on the web (rubato stays native only). Recommendation: create the context at 44100 Hz; fall back to the device rate with the existing resampler only if a browser refuses.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The user has chosen the audio path (A, B, C or another) and the answers to the export, settings and sample-rate questions; the final summary records them with the date
<!-- AC:END -->
