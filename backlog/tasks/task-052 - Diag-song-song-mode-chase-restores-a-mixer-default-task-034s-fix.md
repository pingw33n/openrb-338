---
id: TASK-052
title: 'Diag song: song-mode chase restores a mixer default (task-034''s fix)'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-27 04:14'
updated_date: '2026-10-27 04:27'
labels:
  - tests
  - engine
milestone: m-1
dependencies:
  - TASK-034
references:
  - docs/song_load.md
  - tests/songs/diag/README.md
priority: low
type: task
ordinal: 29700
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
From task-034's review (2026-10-27). task-034 fixed MixerDevice::init_track: the song constructor's slot 8 (0x10011985 → mixer slot 8 at 0x100160F0) inserts a tick-0 event for every automatable mixer controller (1, 2, 6..9 + 6c), so in song mode a chase back to the loop start (loop wrap, locate, song end) restores the constructor default for a controller that has no tick-0 event in the file. No corpus song has that shape: all 163 corpus .rbs files have a tick-0 event for every automatable mixer controller, so the golden test cannot see the fix (it is 164/164 with and without it).

Work: add one diag song to tests/songs/diag/ (spec in gen_specs.py, built by build.py, README row) with: song mode, loop on, loop start 0, a mixer level (controller 6, channel 0) with no tick-0 event and its first event at a later bar setting a non-default value, a 303 or drum playing on that channel so the level is audible, and at least one loop wrap after the event. Remove the tick-0 event for that controller in the built song (the builder may add one by default; say how). Capture its reference with rbhost at --idle-blocks 9088 (tools/rbcorpus.py capture for that one entry; one redc-win-run permit; check `locky list`), add it to tests/refs (FLAC + manifest entry; do not touch other entries), and show that the golden test passes with the task-034 fix and fails without it (revert init_track in a scratch worktree). No app capture is needed (diag references are rbhost renders, decision task-013).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A new diag song with the described shape is built from a spec and listed in the diag README
- [ ] #2 Its rbhost reference (idle 9088) is in tests/refs with a manifest entry; no other entry changes
- [ ] #3 The golden test passes with the fix and fails without it (shown in the notes)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a182ae458bc8e77ac (notes-only; no song added, closed as not feasible).

Finding (2026-10-27): the shape asked for cannot change an export audibly, so no diag reference can show task-034's fix. An export loads the song stopped, runs idle blocks stopped, then stop, locate to loop start (msg 35 {8,start}), play, and renders floor(loop frames) (docs/export_state.md section 4). Chases in that sequence: (a) the export's first stop: halt chases at the current tick regardless of play (seq.rs:358-366), but the live mixer values are still the constructor defaults then; (b) the locate to the loop start: same, the live values are the constructor defaults, because in song mode MixerDevice::data (crates/rb-engine/src/frame/mod.rs ~line 442) copies only the master and channel enables from the MIXR chunk into live, so the live level is 100 whatever the chunk says, and with or without init_track the chase gives 100 for a controller with no tick-0 event; (c) a loop wrap at the loop end falls at or after the last exported frame; (d) when loop_end > 767231 ticks the song end (seq.rs:320, SONG_END seq.rs:28) runs halt + locate(0) (seq.rs:358-380) inside the export range, but halt stops the devices and the output drops to zero, so the restored level is not heard (the reviewer's sweep of 121 loop starts, 744024..767064 step 192, was identical with and without the fix, as was a PCF variant, which does change the output when on). The song GLOB in the file has no play or locate field (tools/rbs.py Glob.FIELDS), so no song can start playing or sit past the event before the export's locate.

Evidence: I built a scratch song (mixer_chase_default: 303 line, song mode, loop on, loop start 0, mixer ch0 level with the tick-0 event dropped via a Song.no_initial set in gen_specs.py, first event at pos 32 = 30, chunk level 50; and a variant at 123456 BPM so the loop length is fractional). rb render of each with the fix and with a scratch copy with the track.insert(0, base + 1, ch.level) line of MixerDevice::init_track removed: WAV sha256 identical in both cases (a20e10a1... for 120 BPM, 742679f6... for 123456 BPM; 176400 and 171461 frames), second half quieter (event level 30) in both. So the golden test would be 165/165 with and without the fix; AC3 cannot be met. Nothing was sent to the test machine.

Recommendation: close as not feasible. The fix is pinned by the unit tests of commit 8e1e072 (frame/tests.rs); a reference could matter only for live playback, which the corpus does not capture.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Closed as not feasible (acceptance criteria not applicable, left unchecked); no song or reference was added. No export shows the mixer chase fix audibly: the export's first stop (halt chases at the current tick, seq.rs:358-366) and its locate to the loop start both find the live mixer values at the constructor defaults, because in song mode MixerDevice::data copies only the master and channel enables from the chunk; the loop wrap falls at or after the last exported frame; with loop_end > 767231 ticks the song end (seq.rs:320, SONG_END seq.rs:28) runs halt + locate(0), but halt stops the devices and the output drops to zero. The file's GLOB has no play or locate field (tools/rbs.py Glob.FIELDS). Evidence: a song with mixer ch0 level without a tick-0 event and a later level-30 event renders to identical WAV sha256 with and without MixerDevice::init_track at 120 BPM (a20e10a1..., 176400 frames) and 123456 BPM (742679f6..., 171461 frames); the reviewer's sweep of 121 loop starts (744024..767064 step 192) and a PCF variant were identical too. The fix is covered by the unit tests of 8e1e072 (frame/tests.rs) and matters only for live playback (locate, stop+locate, loop wrap while playing; task-043's player). Branch worktree-agent-a182ae458bc8e77ac. Testing: none (scratch rb render runs).
<!-- SECTION:FINAL_SUMMARY:END -->
