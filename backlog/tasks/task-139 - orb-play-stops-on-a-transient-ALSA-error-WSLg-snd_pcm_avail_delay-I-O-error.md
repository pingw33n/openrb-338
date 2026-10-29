---
id: TASK-139
title: 'orb play stops on a transient ALSA error (WSLg: snd_pcm_avail_delay I/O error)'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-29 12:19'
updated_date: '2026-10-29 12:48'
labels:
  - cli
  - engine
milestone: m-4
dependencies: []
references:
  - crates/orb-player/src/output.rs
  - crates/orb-cli/src/main.rs
  - crates/orb-ui/src
priority: medium
type: bug
ordinal: 158000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user tested the v2026.1029.1138 release on 2026-10-29. `orb play` of the Linux x86_64 binary under WSL on Windows 11 (WSLg, whose audio goes through PulseAudio, here reached through ALSA) stops at once:

  orb: playing /mnt/c/Rebirth RB-338/Demo Songs/Calm.rbs on Default Audio Device (44100 Hz, 2 channels, f32); Ctrl+C stops
  orb-player: audio output: ALSA function 'snd_pcm_avail_delay' failed with error 'I/O error (5)'
  orb-player: audio output: ALSA function 'snd_pcm_avail_delay' failed with error 'I/O error (5)'
  orb: error: the audio output stopped with an error

crates/orb-player/src/output.rs, build(): the cpal error callback marks the stream failed on any error kind except Xrun and RealtimeDenied, and orb play (crates/orb-cli/src/main.rs, around line 620) quits as soon as Stream::failed() is true. cpal's ALSA backend reports a failed snd_pcm_avail_delay through the error callback and keeps the stream running. The PulseAudio ALSA plugin under WSLg is known to return EIO from it now and then. So one transient error ends playback, though the audio would likely have continued.

What:
- Treat a reported stream error as fatal only when the stream really stopped: for example, when the data callback has not run for a while (about 500 ms) after an error, or after N errors in a row with no data callback between them, or when cpal reports a device-gone kind (DeviceNotAvailable or similar in cpal 0.18). Give the rule in a comment. Log each error at most once per second, with a count, instead of a line per error.
- Check how the UI (orb-ui) uses Stream::failed and the error callback, so the UI and orb play follow the same rule. Ask the user, through the supervisor, whether the UI played sound under WSL.
- Tests: a unit test of the rule with an injected clock or counter (no audio device), covering a transient error that keeps playing, a lasting stop that fails, and a device-gone error that fails at once.
- Real check: this Mac cannot reproduce WSLg. Ask the user to re-run orb play under WSL with the fixed build; that check goes to task-134 as a note or criterion.
- Document in docs/building.md or the orb play help what happens on audio errors, and that under WSL audio goes through WSLg's PulseAudio (ALSA's default device needs libasound2-plugins' pulse plugin).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Unit tests cover the rule without an audio device; fmt, clippy, cargo test --release pass; the WSL re-check is recorded in task-134
- [x] #2 The rule is unit-tested without an audio device (a transient error that keeps playing does not fail; a lasting silence after any non-device-gone error, or a device-gone error, fails with a clear message) and repeated errors are rate-limited in the log; that orb play survives WSLg's errors is pending the WSL re-check in task-134
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. orb-player output.rs: a lock-free rule Health with a caller-supplied clock: device-gone kinds fail at once; every other error kind sets a pending error that fails only when the data callback is silent 500 ms after. Log throttle, one line a second with a count, flushed by the poller. 2. orb play uses Stream::failure(); the UI keeps a Health handle and reports once. 3. Unit tests with injected time. 4. Docs: docs/building.md, play help. 5. fmt, clippy, tests; WSL re-check noted in task-134.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-ab7d3bdf7c6c3c633

Evidence. cpal 0.18.2 src/host/alsa/mod.rs: output_stream_worker (966-1022) calls error_callback for every error and keeps looping, except ErrorKind::DeviceNotAvailable (1015-1018), which reports and returns (worker ends); Xrun (1005-1010) reports and prepare()s. avail_delay failure at 1126-1135 (only EPIPE->Xrun and Suspended are mapped), so EIO becomes a generic error kind (From<alsa::Error> 1713-1716: ENODEV/ENOENT/ENOTSUPP -> DeviceNotAvailable, others BackendError), reported while the data callback keeps being polled. ErrorKind list in src/error.rs:10-85. The old rule failed on every kind but Xrun/RealtimeDenied.

User answer (2026-10-29): under WSL the Linux UI (tar.gz and AppImage) played sound; only orb play stopped. Cause: orb-ui/src/main.rs kept the Stream as _stream and never called Stream::failed(); only orb-cli/src/main.rs:620 polled it. The error callback (eprintln only) was identical in both, so the UI survived by ignoring failed() altogether, and never reported a really lost device. Fix: one shared rule (output::Health); orb play uses Stream::failure(); the UI keeps a Health handle (App::audio_health) and reports a lost device once in the message box (App::check_audio). Health is lock-free (atomics; error text packed into 16 AtomicU64) because tests/no_alloc.rs greps the player sources for Mutex. Linux build check: not run, the x86_64-unknown-linux-gnu std target is not installed here (E0463); Docker not tried.

Review round 1 (send back, 10 points), fixed in the amended commit: show/check_audio doc comments; cpal line cite 1015-1018; the held-back log count is flushed by the poller (Health::flush_log from Health::failure, so orb play and the UI report the tail of a burst a second later); the 500 ms stall backstop now applies to every non-device-gone kind (Xrun, RealtimeDenied, DeviceChanged, DeviceBusy set pending but never fail on the error alone); atomics ordering (last_data, then pending Release; Acquire load of pending first); no_alloc doc says h.data() only stores atomics and the error path allocates; docs and Health comment say a silent stop with no error is not detected and that the UI checks only while it repaints; AC and docs no longer claim the WSL outcome as observed. The branch was rebuilt on main 2b5a028 from the first commit's code files (the backlog files conflicted in a rebase and were redone through the CLI). Checks: fmt, clippy -D warnings, debug cargo test -p orb-player -p orb-ui, cargo test --release --no-fail-fast (615 passed, exit 0).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
orb play and the UI share one rule, output::Health (crates/orb-player/src/output.rs): DeviceNotAvailable/HostUnavailable/StreamInvalidated fail at once; every other error kind sets a pending error and fails the stream only when the data callback is then silent 500 ms (a silent stop with no error is not detected). Logs are one line per second with a count, the tail flushed by the poller. orb play ends with the reason (Stream::failure); the UI, which never looked at failed() (why it survived under WSL), now reports a lost device once in a box. Docs: docs/building.md 'Audio errors and WSL', orb play help. Tests: unit tests with injected time (transient, lasting stop, device gone, soft kinds, log throttle, flush); fmt, clippy, debug tests of orb-player/orb-ui, release --no-fail-fast (615 passed). Not verified under real WSL; the re-check is in task-134. Branch worktree-agent-ab7d3bdf7c6c3c633, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->
