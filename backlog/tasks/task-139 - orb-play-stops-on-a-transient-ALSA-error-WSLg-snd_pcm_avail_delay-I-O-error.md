---
id: TASK-139
title: 'orb play stops on a transient ALSA error (WSLg: snd_pcm_avail_delay I/O error)'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-29 12:19'
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
- [ ] #1 A transient stream error no longer ends orb play or the UI's playback; a lasting stop or a lost device still fails with a clear message; repeated errors are rate-limited in the log
- [ ] #2 Unit tests cover the rule without an audio device; fmt, clippy, cargo test --release pass; the WSL re-check is recorded in task-134
<!-- AC:END -->
