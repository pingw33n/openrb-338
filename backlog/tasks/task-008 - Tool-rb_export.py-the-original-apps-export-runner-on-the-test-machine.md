---
id: TASK-008
title: 'Tool: rb_export.py, the original app''s export runner on the test machine'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 11:16'
labels:
  - tools
  - testmachine
milestone: m-0
dependencies:
  - TASK-002
priority: high
type: feature
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
References come from the original app's File > Export Loop as Audio File (docs/initial_investigation.md section 3). Write `tools/rb_export.py`, a runner that, holding all the permits of `redc-win-run` (doc-001), uploads an .rbs to the test machine, starts ReBirth fresh (no leftover process), opens the song, exports the loop as WAV through the menu and dialogs (rcs_client.py input/screenshot, or a script run on the machine if the machine has a UI automation library; say which), waits for the file to be complete, downloads it, quits the app, and releases the permits. Options for the capture protocol variants the determinism experiment needs: fresh process per song or several exports in one session, play for N seconds before exporting, export twice in a row. It records what it did (protocol, timings) in a JSON sidecar next to the WAV. Use docs/test_machine.md for the UI route.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 rb_export.py exports a given .rbs to a local WAV and JSON sidecar unattended, taking and releasing the permits itself (run on two songs, output listed in the notes)
- [x] #2 The protocol variants (fresh process, repeated export, play before export) are options and recorded in the sidecar
- [x] #3 A failed step (dialog not found, timeout) stops the app it started, releases the permits and exits non-zero with a screenshot saved
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. tools/rb_export.py imports rcs_client.Client (library, not CLI); locky via subprocess. Permits: all of redc-win-run, refreshed each step.
2. Per song: upload .rbs to C:/re/rb/run/, optionally restore prf (from C:/re/rb/prf_before.prf, optional startup-sound patch), start app via cmd start, find new PID with tasklist (refuse if a Rebirth.exe already exists), Enter on MIDI box, Ctrl+O + typed path, wait N s, optional Space play for N s, export route, poll for size stable + equal to expected length formula (docs/initial_investigation.md 3), RIFF check, download, write sidecar JSON, taskkill PID.
3. Options: --reuse-process, --exports K, --play-seconds, --wait-after-open, --restore-prf, --startup-sound.
4. Failure: screenshot, kill own PID, release, exit 1.
5. Test on 2 shipped short songs + 1 rbs.py-built; find the startup-sound byte in the .prf by toggling the checkbox.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a1488bcad521c91c2

Tool: tools/rb_export.py (+ tools/rb_export_test.py, 5 offline tests pass). It uses rcs_client.Client as a library (input/screenshot, no UI-automation lib on the machine), locky by subprocess.
Runs on the machine (2026-10-26):
A) fresh process per song, --restore-prf --startup-sound off --wait-after-open 5: Fonda Workout.rbs -> 302,444 bytes (75,600 frames) sha256 68cff9c3...; G-Spot.rbs -> 2,257,964 bytes (564,480 frames) sha256 d05ad9be...; rc 0, JSON sidecars next to the WAVs.
B) --reuse-process --exports 2 --play-seconds 3 --wait-after-open 3 with a song built by tools/rbs.py (song_mode 1, tempo 128000, loop_end 3072, show_on_open 0) then Fonda Workout: built.1.wav = built.2.wav (1,323,040 bytes, sha 93c99463...), Fonda Workout.1.wav a0d1b52b..., .2.wav 30ca8b4c...; rc 0.
Failure path (AC3): first try of B with a pattern-mode built song: Export greyed -> 'export dialog did not open', screenshot saved (built.fail.bmp), app stopped by PID, permits released, rc 1.
Expected frames from the section-3 formula match the app: Calm 14,003,446 (docs/test_machine.md 4); all 7 exports above had size == 44 + 4*frames.
Failed hypotheses: Export greyed after Space play (no: it was pattern mode, not play); the song-info box needs a blind Enter (no: only when USRI show_on_open). Gaps: WAV only (AIFF type list not driven); startup sound byte 0x20A found by toggling the checkbox, effect on the export not measured (task-010); mouse coordinates unusable (frame position drifts), all keyboard.

Review round 1 (fixes amended into the one commit): retry now sends Enter before replaying the menu; Permits.refresh raises on a failed locky refresh; one Machine.sleep helper ticks the lease in all long waits (wait-after-open, play-seconds, start); stop_app raises Failure if the PID survives taskkill /F; the machine's prf_before.prf is checked against the documented SHA-256; stray-file cleanup removed (and its sidecar key); dead STARTUP_SOUND_OFFSET None branch removed; files opened with 'with'.
AC3 evidence correction: the pattern-mode failure run quoted earlier came from an earlier revision; the committed code refuses pattern-mode songs before starting the app. New run of the committed code with the added --test-fail option (forced 3 s dialog timeout after the app started and the song opened): 'FAILED: window ... did not appear within 3 s', screenshot saved (o5/Fonda Workout.fail.bmp), 'app stopped (taskkill)' for pid 13156, 'released the permits (ok)', rc 1; locky shows 6/6 available afterwards. Normal export re-run after the changes (--restore-prf --startup-sound off): Fonda Workout 302,444 bytes, rc 0. Unit tests 5 pass with -W error::ResourceWarning.

Review round 2: only the commit message was stale; amended to describe the committed code and the round-1 runs. No code change.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
tools/rb_export.py exports .rbs songs through the original app unattended: takes all redc-win-run permits (refresh/release, also on failure), restores the preferences optionally (+ startup-sound byte 0x20A), starts Rebirth.exe by PID, opens the song, dismisses the info box when USRI show_on_open, waits T, optional Space play, exports K times, verifies size = 44+4*frames and RIFF, downloads, writes a JSON sidecar (protocol, timings, hashes), stops only its PID. Options: --restore-prf, --startup-sound, --wait-after-open, --play-seconds, --exports, --reuse-process. Tested on Fonda Workout and G-Spot (fresh) and a rbs.py-built song + Fonda (reused process, 2 exports, play 3 s); failure path tested. docs/test_machine.md section 8 records new facts (Export greyed in pattern mode, info box, prefs byte). Branch worktree-agent-a1488bcad521c91c2, one commit. Tests: unittest tools/rb_export_test.py 5 pass; machine runs above.
<!-- SECTION:FINAL_SUMMARY:END -->
