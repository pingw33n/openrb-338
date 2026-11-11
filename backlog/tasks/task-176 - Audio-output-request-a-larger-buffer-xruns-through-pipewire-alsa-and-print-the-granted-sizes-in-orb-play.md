---
id: TASK-176
title: >-
  Audio output: request a larger buffer (xruns through pipewire-alsa) and print
  the granted sizes in orb play
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-11-11T15:49:00Z'
labels:
  - cli
  - ui
milestone: m-5
dependencies: []
references:
  - crates/orb-player/src/output.rs
  - crates/orb-cli/src/main.rs
  - https://github.com/pingw33n/openrb-338/issues/7
priority: high
type: bug
ordinal: 192000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: GitHub issue #7 (2026-11-11). On Linux Mint (Dell Inspiron 16 5640, PipeWire 1.0.5; ALSA's `default` is the pipewire-alsa plugin) the release 2026.1103.0917 x86_64 plays with constant crackling: `orb play` opens "Default Audio Device (44100 Hz, 2 channels, f32)" and cpal reports ErrorKind::Xrun ("A buffer underrun or overrun occurred.") 23-24 times a second for as long as it plays; OpenRB-338 (the UI) logs the same stream and the reporter hears nothing usable. A Manjaro user reports no sound too; a Debian user (arm64, amd64) reports it works. It is not CPU: `orb render` of the whole song takes 0.44 s on that machine. PIPEWIRE_LATENCY=1024/44100 and PULSE_LATENCY_MSEC=60 change nothing.

Likely cause (inferred, not yet confirmed on the reporter's machine): orb-player opens the stream with cpal's BufferSize::Default (crates/orb-player/src/output.rs, `build`). In cpal 0.18.2's ALSA host (src/host/alsa/mod.rs:1642-1656) Default means: take the period size pipewire-alsa picks, then cap the ring buffer at DEFAULT_PERIODS = 2 periods. When PipeWire's graph pulls a bigger block per cycle than 2 periods hold, every cycle underruns: 48000 / 2048 = 23.4 cycles a second matches 23-24 errors a second if the graph runs at a 2048 quantum at 48 kHz. The reporter was asked to confirm with `pw-top` and `pw-metadata -n settings 0 clock.force-quantum 256` (undo with 0); record the answer here if it arrives.

What:
- output.rs `start`/`choose`: request BufferSize::Fixed(n), n = 2048 frames (cpal makes the ALSA ring 2n, about 93 ms at 44.1 kHz), clamped into the device's SupportedStreamConfigRange::buffer_size() when it is a Range; when the range is Unknown or the build with Fixed fails, fall back to BufferSize::Default (do not fail where Default would have worked). Keep it for every host (CoreAudio and WASAPI take it as a request too); check that the macOS and Windows paths still open (cargo test, and orb play on the user's Mac).
- The engine's timing is unchanged: commands still apply per 1024-frame engine call (crate doc of orb-player); the device buffer only adds latency. Document the chosen size and why in output.rs's module doc.
- Stream (output.rs) records the buffer size requested and granted (for ALSA the period/buffer cpal configured if obtainable; otherwise the first data callback's frame count), and the data callback tracks the largest frames-per-callback and the longest callback duration in atomics (no allocation or lock in the callback; tests/no_alloc.rs stays green).
- `orb play`'s start line (crates/orb-cli/src/main.rs, fn play) adds the buffer: e.g. "on Default Audio Device (44100 Hz, 2 channels, f32, buffer 2048 frames requested, 2048 granted)"; on exit (Ctrl+C, end, or failure) it prints one line with the callback frame sizes seen (min/max) and the longest callback in ms. OpenRB-338 prints the same start line to stderr at launch.
- Out of scope: cpal's native PipeWire host (feature `pipewire`, links libpipewire, changes packaging): if the Fixed buffer does not cure the xruns, write a Decide: task for it with the evidence instead.
- docs/building.md or README's Linux notes: one line on the xrun symptom and what to report (the orb play lines).

Not reproducible on the macOS dev machine or in CI (no audio device); the verification on PipeWire is the reporter's (issue #7) unless a Linux machine with PipeWire is available. Prepare a short reply text for the issue asking them to run the new build's orb play and paste both lines.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 orb-player requests a 2048-frame buffer, clamped to the device's supported range, and falls back to the default buffer when the range is unknown or the request is refused; a unit test covers the size choice (supported range, unknown range, out-of-range clamp)
- [ ] #2 orb play's start line names the requested and granted buffer, and its exit line the min/max frames per callback and the longest callback in ms; OpenRB-338 prints the start line at launch; the callback still allocates nothing (tests/no_alloc.rs)
- [ ] #3 orb play works unchanged on macOS (the user's machine) with the new line shown; fmt, clippy and cargo test --release pass
- [ ] #4 The task notes hold a reply text for issue #7 asking the reporter to run the new build's orb play and paste both lines
<!-- AC:END -->
