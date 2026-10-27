---
id: TASK-083
title: 'Song load: msg 37 refuses a batch larger than the pool''s free count'
status: To Do
assignee: []
created_date: '2026-10-27 17:07'
labels:
  - engine
  - re
milestone: m-3
dependencies:
  - TASK-082
references:
  - crates/rb-engine/src/seq.rs
  - docs/initial_investigation.md
  - docs/song_load.md
priority: low
type: enhancement
ordinal: 104000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found by task-082 (2026-10-27) in ReBirth.dll: song msg 37 (sub_10021010) reads the batch header [track, u32 first, u32 last] (sub_10019EB0), computes count = last - first (0x10021042) and compares it with the pool's free count (sub_1000DEE0 = [dword_10100230+4], 0x10021054). When count is above the free count (cmp ebx, eax; ja at 0x1002105B) it inserts nothing and replies through sub_1001DE00 with 0x16 (22) instead of sub_1001DF80 with 0x49 (73) (0x100210ED vs 0x100210CA). The exe sends a track's events in batches of at most 1284 (docs/song_load.md section 5, sub_467E70; section 12.2 for 1.x songs) and treats a failed reply as out of memory ('K', song_load.md).

rb's Sequencer::add_events (crates/rb-engine/src/seq.rs) inserts every event of the batch until sub_10014434's per-node refusal (one or no node free), and the load gives add_events a whole track, not 1284-event batches. The two agree whenever a song has fewer than about 100000 events; they differ only for a song that fills the pool.

Do: find what the exe does with the 22 reply (song_load.md's 'K' path: does the load abort, and what does the user get), then model the batch split and the refusal in the load (crates/rb-engine/src/frame/mod.rs Engine::load) and add_events, or record why not. Needs a song of more than 100000 events to test (a generated one; compare against rbhost on the DLL if the load path is reachable there).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The exe's handling of the msg 37 refusal is documented with addresses in docs/song_load.md
- [ ] #2 rb's load splits the tracks into the exe's batches and refuses a batch as sub_10021010 does, with a unit test, or the notes say why it is not modelled
- [ ] #3 The golden test and compare (164/164) pass
<!-- AC:END -->
