---
id: TASK-080
title: 'Decide: save the engine''s tracks always, as the original does'
status: Done
assignee:
  - '@pingw33n'
created_date: '2026-10-27 16:28'
updated_date: '2026-10-29 14:27'
labels:
  - decision
  - ui
milestone: m-3
dependencies: []
references:
  - docs/ui_spec.md
  - crates/rb-ui/src/document.rs
priority: low
type: spike
ordinal: 102000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Context (task-057): the original's save writes the TRAK chunks from what it reads back from the DLL (exe sub_437CFF asks with song msg 22 {track, from, from+1284} at 0x437DE6, writes each tick as 8*tick/192 at 0x437E9E). So an original-saved song always has the engine's tracks: the song ctor's tick-0 events of every controller (every shipped song has exactly 18, 9, 9, 29, 30, 5, 3, 7, 3 events at position 0 in tracks 0..8), the stop prune's removals (sub_100149F0 unlinks an event that repeats its controller's previous value, at every stop), the insert's quantisation and same-tick dedup, and any recording.

rb-ui (task-047) instead keeps the file's track events as read, so a song saved without an edit is the file byte for byte; task-057 made it write the engine's tracks only once recording has merged events into them (Link::recorded_tracks, from rb_player::Player::tracks).

Options:
A. Keep task-057's rule (file's tracks unless recorded). Unedited saves stay byte-identical; a recorded song's file matches what the original would write for the same engine state.
B. Always write the engine's tracks (rb_player::Player::tracks is already published after every load, merge and stop), as the original does. Saved files then match the original's for every song, but an unedited save of a file that lacks tick-0 events or has repeated values changes the file (the menu test saving_an_unedited_corpus_song_gives_back_the_file would need the corpus to be original-saved or would compare against the engine's tracks).

Recommendation: A for now (no visible difference in what plays; the byte-identity of unedited saves is a useful property and test), revisit if files saved by rb must equal the original's byte for byte.
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
User, 2026-10-29: the decision is changed to option B. OpenRB-338 always saves the engine's tracks, as the original does (sub_437CFF reads them back with song msg 22), not the file's own events. The implementation is task-143. Context: 35 songs of the third-party collection carry events past bar 999 that the original drops on load and so never saves; an unedited OpenRB-338 save kept them, along with repeated values the original's stop prune removes.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Decided by @pingw33n on 2026-10-27: option A, keep rb's rule. A save writes the song file's own track events unless recording changed them, then the engine's tracks (task-057); an unedited song saves back byte for byte. The original's always-the-engine's-tracks behavior (sub_437CFF reads them back with song msg 22) is not copied.
<!-- SECTION:FINAL_SUMMARY:END -->
