---
id: TASK-034
title: 'RE: engine side of song creation and song-mode defaults'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-26 10:05'
updated_date: '2026-10-27 04:12'
labels:
  - re
  - docs
milestone: m-1
dependencies:
  - TASK-003
references:
  - docs/song_load.md
  - docs/initial_investigation.md
priority: low
type: spike
ordinal: 34000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
task-003 traced how Rebirth.exe turns an .rbs into engine messages (docs/song_load.md). Four engine-side points were left open (docs/song_load.md section 11) and matter for the song model (task-018) and the export start state:
1. Top-level msg 52 {u16 mod} opens a new song; the DLL queues it (sub_1001D694 -> object at +0x414, sub_10009520 / sub_10026EA0). Find where it is processed, whether it creates new device objects (so fresh 303, effect and mixer state at every load), and the constructor defaults of every device parameter.
2. In song mode the DLL does not copy a device's msg-41 parameters to the live values (303 at 0x10025CB0, mixer at 0x1001588F); a controller with no TRAK event at or before the locate keeps its constructor value. Confirm the same rule in the delay (0x1000FB40), PCF (0x10016C50), distortion (0x10010D60), compressor (0x1000EFD0), 808 (0x100282B0) and 909 (0x1002B8E0) host-message handlers, and list the defaults that apply.
3. The 4.0 mixer-track converter sub_437380 in the exe (4.0 files only; RBS42 is 4.2): its controller remapping.
4. Global byte 2 (msg 35 id 2, setter sub_10021190, a thunk to 0x10021194, used by locate 0x1001F984) and byte 22 (song +0x16E): their meaning.
Read-only IDA work on headless copies (doc-001); write the results into docs/song_load.md with addresses.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 docs/song_load.md states, with addresses, whether msg 52 creates new device objects and lists the constructor default of every automatable parameter
- [x] #2 The song-mode msg-41 rule is confirmed or refuted for each of the six other devices
- [x] #3 sub_437380's controller mapping and the meaning of global bytes 2 and 22 are documented, or marked unresolved with what was tried
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Collect what later tasks answered: msg 52 (task-004, export_state.md 3), the msg-41 rule for all devices (task-022/024/025/026/027, song_load.md 4), sub_437380 (task-018, song_load.md 5.1), byte 2 (task-022).
2. In a headless copy of ReBirth.dll.i64: re-check the msg 52 path (sub_1001D694 -> sub_1001D840 -> ctor sub_10011350), and read every device ctor and its parameter-default function (mixer, 303, 808, 909, delay, dist, PCF, comp) from the disassembly; check both copies are initialized and whether slots 7/8/9 change them.
3. Search the DLL for readers of song+0x16E (byte 22).
4. Compare with crates/rb-engine DEFAULT_PARAMS / DEFAULTS / MixerParams::default; add a unit test pinning the table; fix only on a mismatch with the golden test at 164/164.
5. docs/song_load.md: a defaults table (device, parameter, controller, default, address), byte 22, section 11 updated; original_bugs.md if a defect turns up.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From task-004 (2026-10-26): item 1 is answered in part: docs/export_state.md section 3 shows every load (msg 52) creates new device objects [C], so no per-song state survives a load. The constructor default of every automatable parameter (the rest of item 1) and items 2-4 remain.

Branch: worktree-agent-aaa53f5dff9613ea7

Collected from later tasks (no new RE needed): msg 52 creates new device objects (task-004, export_state.md 3: sub_1001D694 queue, EngineIdle sub_1001D840, ctor sub_10011350); the song-mode msg-41 rule for the 808 (task-024, 0x1002884C), 909 (task-025, 0x1002BE90), delay 0x1000FD7D and distortion 0x10010F77 (task-026), PCF 0x10016EA5 and compressor 0x1000F1DF (task-027) - all confirmed in song_load.md 4; sub_437380 fully traced (task-018, song_load.md 5.1); global byte 2 = record (task-022).

IDA (headless copy of ReBirth.dll.i64, read-only, closed save=false). Parameter-default functions read store by store: 303 sub_10018C20 = 1,0,64,20,90,20,100,64,0,0,0,0; 808 sub_10018FB0 = 1,0,40,100,64,64,100,64,64,100,64,0,100,64,0,100,64,0,100,0,100,0,100,100,0,100,100,100,100,0,0,0; 909 sub_100195A0 = 1,0,32,100,64,64,127,100,64,64,64,100,64,127,100,64,127,100,64,127,100,100,100,32,127,100,64,100,64,64,0,0,0; delay sub_10019AB0 = 1,3,1,80,64,0; dist sub_10019B30 = 1,70,0,0; comp sub_10019B90 = 1,70,64,127,0; PCF sub_10019C10 = 1,70,64,40,24,90,1,0; mixer sub_10019CC0 master 100, comp id dword +4 = 0, PCF id dword +8 = 0, +0xC/+0xD 0, and sub_10019D50 per channel 1,100,64,0,0,0. Each has exactly two xrefs, both in its device ctor (one per block: live and pattern copy, call addresses in song_load.md 4.9). All equal the engine's DEFAULT_PARAMS / DEFAULTS / MixerParams::default.

Finding: song ctor slot 8 (0x10011985) inserts a tick-0 track event of every automatable controller on EVERY device. Vtable slot 8: delay 0x10010020 (ctrl 0..4), dist 0x10010FE0 (0..2), comp 0x1000F420 (0..2), PCF 0x10017200 (0..6), 909 0x1002D480 (0..29), 303 0x10025650, 808 0x10029DA0, and the mixer 0x100160F0 (ctrl 1 from +0x5C, 2 from +0x60, then per channel 6..9+6c from +0x69..+0x6C+6c). Slot 7 is nullsub 0x1000F4E0 for all but the 303/808/909. The engine modelled every slot 8 except the mixer's: MixerDevice used the trait's no-op init_track. Effect: in song mode, a mixer controller with no tick-0 event in the song keeps a later event's value after the position goes back (loop wrap, locate) instead of returning to the ctor default. Fixed: MixerDevice::init_track in crates/rb-engine/src/frame/mod.rs. Test song_mode_chase_restores_the_mixer_ctor_values fails without it (left 50, right 100; checked by disabling the inserts) and passes with it.

Byte 22 (song+0x16E): globals ctor sub_10018AC0 sets 1 (0x10018AEA); writers msg 35 id 22 (case at 0x100126A1, store 0x100126A7) and msg 36 (0x100127E0); only reader the msg 21 pack sub_10018B00 (0x10018B8B). llvm-objdump of original/ReBirth.dll: no other +0x16e displacement; byte [reg+0x16] accesses only in globals funcs and other structs' parameter blocks; msg 20 query switch covers ids 0..2 only (0x10011FC2). So no engine effect [C]; the exe's meaning stays [G].

Tests: cargo test --release all pass (rb-engine lib 162, incl. the two new frame tests); corpus_golden 4/4 = 164/164 (145 diag + 11 app + 8 shipped) byte-exact, unchanged; cargo clippy --all-targets -D warnings clean; cargo fmt clean. Docs: song_load.md 3 (byte 22), 4 (rule for all devices), new 4.9 (defaults table + slot 8), 6 (chase), 11 (open items); initial_investigation.md 'Song ctor slots 7 and 8'. No defect of the original found (original_bugs.md unchanged).

Review round 1 (send back, wording): (1) song_load.md 6 said every shipped song has a tick-0 event for 'every accepted controller'; now 'every automatable controller (for the mixer 1, 2 and 6..9+6c)', noting that 10, 11, 16, 17, 22, 23 (a channel's unused byte and enable) get no ctor insert and setParam ignores them; the same qualifier added in 4.9 and 6 where 'automatable' is used. (2) Added init_track_holds_the_ctor_defaults to tr808/tests.rs and tr909/tests.rs, pinning the literal 4.9 default tables (808 controllers 0..28, 909 0..29 in RBS order); the frame test's doc comment now names them. (3) Assignee set to @agent-complex. cargo test --release all pass (rb-engine lib 164), golden 164/164 unchanged, clippy and fmt clean. Note: the two drum tests were appended with a shell heredoc rather than the Write tool.

Review round 2: (1) MixerDevice::set_param doc comment (crates/rb-engine/src/frame/mod.rs) said track 0 accepts 1, 2 and 6..27; corrected to the exe's 1, 2 and 6..9+6c (sub_42F8F0, song_load.md 5), which load::control_range already implements (6..27 with (ctrl-6)%6 in 0..3), so no code change. (2) Rewrapped the long lines this task added in docs/song_load.md 6 and 11 to 100 columns. cargo test --release all pass, golden 164/164.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Engine side of song creation is fully documented. msg 52 builds new device objects at every load (task-004, confirmed path). The constructor default of every automatable parameter of all nine devices was read from the disassembly and tabled with addresses in docs/song_load.md 4.9; all equal the rb engine's constants, now pinned by unit tests for every device. The song-mode msg-41 rule holds for all devices (task-024..027 evidence collected in song_load.md 4). sub_437380 was traced by task-018; byte 2 is record (task-022); byte 22 (song+0x16E) has no engine effect (only echoed by the msg 21 reply). New finding: the song ctor's slot 8 inserts a tick-0 event of every automatable controller on every device; rb lacked the mixer's (0x100160F0), so a song-mode mixer controller without a tick-0 event in the song did not return to its default when the position went back. Fixed with MixerDevice::init_track plus unit tests. Review round 1 fixed wording and pinned the drum defaults. Branch worktree-agent-aaa53f5dff9613ea7, one commit. cargo test --release all pass (rb-engine lib 164), golden 164/164 byte-exact (unchanged), clippy and fmt clean.
<!-- SECTION:FINAL_SUMMARY:END -->
