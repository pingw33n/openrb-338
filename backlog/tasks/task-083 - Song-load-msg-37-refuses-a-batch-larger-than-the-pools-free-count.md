---
id: TASK-083
title: 'Song load: msg 37 refuses a batch larger than the pool''s free count'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-27 17:07'
updated_date: '2026-10-27 20:37'
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
- [x] #1 The exe's handling of the msg 37 refusal is documented with addresses in docs/song_load.md
- [x] #2 rb's load splits the tracks into the exe's batches and refuses a batch as sub_10021010 does, with a unit test, or the notes say why it is not modelled
- [x] #3 The golden test and compare (164/164) pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Document the refusal path in docs/song_load.md (section 5 and section 1's reply table), initial_investigation.md's pool item, and the leaked partial song in docs/original_bugs.md.
2. song::Song gets batches: the exe's msg-37 send order as (track, accepted count), filled by load::load and load_1x from Loaded.order.
3. Sequencer::add_events refuses a batch longer than the pool's free count (nothing inserted, returns false), as sub_10021010.
4. Engine::load sends the tracks in those batches (leftover events of an edited song go as one more batch per track; an empty plan = one batch per track) and stops at a refused batch without msg 32, returning Err(LoadRefused) (kept also on the engine). export_song returns Result; rb render fails with the exe's alert text; Player::build keeps the engine (follow-up task for rb-ui / rb play to show the alert).
5. Unit tests: add_events refusal at count > free, count == free (last node refused per node), Engine::load split and stop; load tests for batches. Then cargo test, clippy, fmt, golden test and compare 164/164.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a2b0677fea8fa9b4c

Research (IDA, headless copies of ida/ReBirth.dll.i64 and ida/Rebirth.exe.i64, closed save=false):
- DLL sub_10021010 disassembly: count = last - first (sub ebx, edx at 0x10021042), free = sub_1000DEE0 (mov eax,[ecx+4]) on dword_10100230 (0x10021054), cmp ebx,eax; ja 0x100210ED (unsigned). Passed: loop of sub_10014750 inserts (0x1002106F..0x100210C8), then sub_1001DF80(0x49) (0x100210CA). Refused: sub_1001DE00(0x16) (0x100210ED), nothing inserted.
- sub_1001DF80 (sub_10018100) builds a one-byte packet [0x49 'I']; sub_1001DE00 (sub_10018110 with type 0x4B 'K', 1 byte) builds [0x4B 'K', 22]: the refusal is the 'K' (out of memory) reply, code 22.
- Exe sub_436B6F 0x436E41..0x436E95: sub_40ABE5 reads the reply; Src[0] == 75 throws XOutOfMemory. Its catch(...) at 0x436EEA deletes the stream and rethrows. sub_434D05's catch(...) 0x436AD7 deletes its stream and rethrows (push 0; push 0; call _CxxThrowException at 0x436B20). sub_425C4D's catch(XOutOfMemory) at 0x426430 (try 0x4263B2, RBS branch) throws XSongOutOfMemory (vftable 0x4BBD9C, TI 0x4C64A0); same for the 1.x branch (catch 0x425FDB, try 0x425F74; sub_430163 throws XOutOfMemory at 0x430315). So msg 32 (0x426610) is never sent. Unwind of sub_425C4D: member +8 dtor 0x425900 (empty), stream sub_47D7AB; no message to the DLL.
- sub_40B575 catch(XSongOutOfMemory) 0x40B778: sub_475D5C(0x1DCF592A) = CAlertBox (sub_4754B9) with text id 500128042 (sub_47B074: group 128, item 42 of the SALR text) = '{[STOP]There is not enough memory for opening the Song.[OK]}' (exe file offset 0x159897), then rethrow; unwind deletes the 0x338 document; the song window (sub_42DA72) is never built. sub_40A2A3's catch(...) 0x40A3D4 (File > Open, dialog 0x17D97802) swallows it and returns 0.
- No close message for the song slot from msg 52 on any of these unwinds: the DLL song keeps the earlier batches' nodes (leak in the shared pool until the engine closes).

Implementation:
- song::Batch and Song::batches (send order, header count last - first), filled by load::load and load_1x from Loaded.order (load::send_plan).
- Sequencer::add_events -> bool (#[must_use]): refuses when events.len() > pool.free(), nothing inserted, revision unchanged.
- Engine::load -> Result<(), LoadRefused> (frame/mod.rs): walks Song::batches, then whatever a batch plan leaves of each track as one more batch (an empty plan = one batch per track; an empty batch is not sent, as sub_436B6F/sub_430163); stops at the refused batch with no msg 32 (relocate), keeps it (Engine::load_refused).
- export_song -> Result; rb render fails with 'There is not enough memory for opening the Song (...)'. Player::build keeps the engine (let _ = e.load) and documents it; follow-up task-088 for rb-player/rb-ui/rb play.
- Failed hypotheses in the tests: a fill with repeated values on a stopped song was pruned by msg 36's stop (the load then passed), fixed with play = 1; the 101000-event file was refused at its last partial batch (1196 of track 6), not a full one, as the free count after 78 full batches was below 1196.
- Not run on the test machine: the exe never opens such a song, so there is no reference render; the DLL check and the exe path are read from the disassembly (82 instructions of sub_10021010, the catch and unwind funclets listed above).
- Corpus: the largest RBS file of tests/songs has 13003 track events (Mutilated Species II, tools/rbs.py count), so no corpus song comes near the pool.

Results: cargo test --release (RB_STANDARD_MOD, RB_MOD_DIR set) 444 passed, 0 failed; cargo test --release -p rb-engine --test corpus_golden ok (4 passed); rb compare --set diag,app,shipped: 164 of 164 equal; clippy --all-targets -D warnings clean; fmt clean. New unit tests: seq msg37_refuses_a_batch_above_the_free_count, frame load_stops_at_a_refused_batch and a_file_above_the_pool_is_refused (a generated 101000-event RBS model: refused at track 6's last batch, first 39804, count 1196), load the_song_keeps_the_batch_plan; message_stream checks the plan.

Review round 1 (merge verdict with doc nits), fixed in the amended commit:
1. song_load.md 5.2: 0x10021054 is the call to sub_1000DEE0 (body mov eax,[ecx+4]; ret), now said so.
2. 'When it happens': the Untitled song cited to docs/export_state.md section 3 (msg 53) and the ctor's pool nodes to initial_investigation.md section 5 (task-082); the node count marked [G].
3. Re-read every address of section 5.2 and OB-20 with llvm-objdump on original/ (script scratchpad/task-083/objdump.sh: DLL 0x10021036..0x10021106, 0x1000DEE0, 0x1001DE00, 0x1001DF80; exe 0x436E41..0x436E95, 0x436EEA..0x436F37, 0x436AD7..0x436B24, 0x430300..0x430315, 0x426423..0x42645B, 0x425FDB..0x42600F, 0x426600..0x426610, 0x40B778..0x40B789, 0x40A2ED..0x40A310, 0x40A3D4, 0x425900 (stores ecx, returns: empty), 0x4B5720..0x4B572E). All matched, with one correction: the exe compares the reply with 0x49 at 0x436E52 and 0x4B at 0x436E65 (0x436E55 is the jne after the 'I' test), and in sub_430163 the 'K' test is at 0x430305 (0x430315 pushes the throw info); song_load.md, OB-20 and the LoadRefused doc now cite those.
4. OB-20's rb field says rb accepts some songs the exe refuses (one pool per song).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Documented and modelled the msg 37 batch refusal.

Findings (docs/song_load.md section 5.2, [C]): sub_10021010 refuses a batch whose header count (last - first) is above the pool's free count (ja at 0x1002105B) with the packet ['K', 22] (sub_1001DE00) instead of ['I'] (sub_1001DF80). The exe throws XOutOfMemory (0x436E95; 1.x 0x430315), sub_425C4D turns it into XSongOutOfMemory (0x426430 / 0x425FDB), sub_40B575 shows the STOP alert 'There is not enough memory for opening the Song.' (0x40B778, alert id 0x1DCF592A = SALR group 128 item 42) and File > Open (sub_40A2A3, catch(...) 0x40A3D4) opens nothing; no later batch and no msg 32 are sent. No unwind closes the engine's song, so its earlier batches keep their pool nodes: new OB-20 in docs/original_bugs.md. initial_investigation.md's pool item updated.

Model: song::Song::batches (send order, header counts) from load::load and load_1x; Sequencer::add_events refuses (returns false) a batch above the free count; Engine::load sends the batches, stops at a refused one and returns/keeps LoadRefused; export_song returns Result and rb render fails with the alert text. Player::build still returns the engine (follow-up task-088 for rb-player, rb-ui and rb play).

Tests: cargo test --release 444 passed; corpus_golden ok; compare 164/164 equal; clippy and fmt clean. Not run on the test machine: the exe never opens such a song, so there is no reference render.

Branch worktree-agent-a2b0677fea8fa9b4c, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->
