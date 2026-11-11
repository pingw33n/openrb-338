---
id: TASK-176
title: >-
  Audio output: request a larger buffer (xruns through pipewire-alsa) and print
  the granted sizes in orb play
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-11-11T15:49:00Z'
updated_date: '2026-11-11T16:10:00Z'
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
- [x] #1 orb-player requests a 2048-frame buffer, clamped to the device's supported range, and falls back to the default buffer when the range is unknown or the request is refused; a unit test covers the size choice (supported range, unknown range, out-of-range clamp)
- [x] #2 orb play's start line names the requested and granted buffer, and its exit line the min/max frames per callback and the longest callback in ms; OpenRB-338 prints the start line at launch; the callback still allocates nothing (tests/no_alloc.rs)
- [x] #3 orb play works unchanged on macOS (the user's machine) with the new line shown; fmt, clippy and cargo test --release pass
- [x] #4 The task notes hold a reply text for issue #7 asking the reporter to run the new build's orb play and paste both lines
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. output.rs: pure fn buffer_request(range) (2048 clamped; Unknown -> Default); choose returns the range; Stats (atomics: first/min/max frames, longest callback ns) updated in the data callback closure (no alloc/lock); start tries Fixed then falls back to Default, recovering Audio from the failed closure via a Drop guard (Mutex only touched on drop). Stream gets requested/granted/stats accessors; granted = first callback's frame count (cpal ALSA exposes no period getter). 2. Unit tests for buffer_request. 3. orb play start/exit lines, orb-ui launch line on stderr. 4. building.md Linux note. 5. fmt, clippy, tests, run orb play on the Mac.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: task-176-audio-buffer

Evidence. cpal 0.18.2 ALSA host (src/host/alsa/mod.rs): Fixed(x) = period x, ring 2x (1600-1636), errors when x is outside the period range (so the request is clamped first); Default = plugin period, ring capped at 2 periods (1642-1656). Implemented: pure buffer_request(range) = 2048 clamped, Unknown -> Default; Stream tries Fixed(n), then Default on refusal; the Audio is recovered from the dropped callback through a one-place rtrb queue (a Mutex first failed tests/no_alloc.rs the_callback_crates_hold_no_lock, which bans the word). Granted size = first data callback's frame count (cpal has no period getter); callback stats are atomics recorded in the callback closure (min/max/first frames, longest callback ns). start() waits up to 300 ms for the first callback so the start line shows the granted size. Verified on the Mac (MacBook Pro Speakers, CoreAudio): 'orb play tests/songs/shipped/Calm.rbs --seconds 3' printed: orb: playing ... on MacBook Pro Speakers (44100 Hz, 2 channels, f32, buffer 2048 frames requested, 2048 granted); Ctrl+C stops / orb: audio callbacks: 66 callbacks, 2048..2048 frames each, longest 0.30 ms (granted 2048 frames). Fallback path exercised with a temporary hack that failed the Fixed build: it played on the default buffer (line read 'default size requested, 2048 granted'); hack reverted. Failed hypothesis: channels=99 does not make CoreAudio refuse the Fixed build. Limits: no Linux/PipeWire machine, so the xruns were not reproduced and the fix is unverified; ALSA's first-callback size may differ from the period (cpal passes avail frames). Ctrl+C kills orb play with no handler, so the exit line is not printed then (no signal crate in the tree; adding ctrlc is a dependency choice left out); docs/building.md says to use --seconds. The OpenRB-338 line goes to stderr at launch. If the Fixed buffer does not cure the xruns, a Decide: task for cpal's pipewire host is the next step (out of scope here).

Reply text for issue #7 (not posted): Thanks for the detailed report. The audio output asked ALSA for the default buffer, which behind pipewire-alsa is only two periods, so a larger PipeWire cycle (48000/2048 = 23.4 a second matches your 23-24 xruns) underruns every cycle. The next build asks for a fixed 2048-frame buffer (a ring of about 93 ms). When that build is out, please run: orb play SONG.rbs --seconds 10 (any song) and paste both lines it prints: the one starting 'orb: playing ...' (it names the buffer requested and granted) and the one starting 'orb: audio callbacks: ...' (callback sizes and the longest callback), and tell us whether the crackling is gone. If it is still there, please also paste the output of pw-top during playback and of: pw-metadata -n settings 0 clock.force-quantum 256 (undo with the value 0). Manjaro users: the same two lines help.

Review round 1 (supervisor, send back): (1) Ctrl+C must print the exit line; (2) drop the Ctrl+C sentence from docs/building.md and rewrap; (3) do not call the first callback's frame count 'granted'. Decision (user, 2026-11-11): add the ctrlc crate to orb-cli. Done: ctrlc 3.5.2 in crates/orb-cli (Cargo.lock updated; it supports Linux, macOS, Windows); the first Ctrl+C sets an AtomicBool that the play loop checks, so the stream is dropped and the exit line printed, exit 0; a second Ctrl+C exits with 130. Checked with a python subprocess sending SIGINT after 3 s: exit 0 and 'orb: audio callbacks: 61 callbacks, 2048..2048 frames each, longest 0.29 ms' (a shell '&' job ignores SIGINT, so kill -INT there tests nothing). The start line now reads 'buffer 2048 frames requested, first callback 2048 frames' (no 'granted'; caveat in the doc comment of Stream::buffer_text: on ALSA the first callback is the frames available, not necessarily the period); the exit line lost its '(granted ...)' tail. Revised reply text for issue #7: replace 'it names the buffer requested and granted' by 'it names the buffer requested and the size of the first callback'; the rest stands (orb play SONG.rbs --seconds 10 or Ctrl+C after about 10 s now prints the exit line too). The earlier note 'Ctrl+C ends without the exit line' is obsolete.

Round 1 re-run: cargo test --release no failing result, clippy -D warnings and nightly fmt --check clean; kill -INT of a backgrounded orb play also printed the exit line (63 callbacks, exit 0).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
orb-player now requests BufferSize::Fixed(2048) clamped into the device range (Unknown range or a refused build falls back to Default); Stream reports requested/granted and callback stats; orb play and OpenRB-338 print the start line, orb play an exit line (not on Ctrl+C); docs/building.md has the Linux note. Branch task-176-audio-buffer. Tests: cargo test --release all 72 result lines ok (unit tests buffer_choice, callback_stats; no_alloc green), clippy -D warnings and nightly fmt --check clean, orb play run on the Mac. The PipeWire fix is unverified (no Linux machine); verification is the reporter's, reply text in the notes.
<!-- SECTION:FINAL_SUMMARY:END -->
