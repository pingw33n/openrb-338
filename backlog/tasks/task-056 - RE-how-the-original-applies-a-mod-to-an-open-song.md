---
id: TASK-056
title: 'RE: how the original applies a mod to an open song'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-27 07:34'
updated_date: '2026-10-27 14:40'
labels:
  - re
  - rbm
  - ui
milestone: m-3
dependencies:
  - TASK-043
references:
  - docs/rbm.md
  - docs/ui_spec.md
  - crates/rb-player/src/lib.rs
priority: medium
type: task
ordinal: 51000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the Mods menu (docs/ui_spec.md section 12: "Select Mod...", one item per mod) applies a mod to the open song. rb knows only that a song is created with its sample set (msg 52, crates/rb-engine/src/song.rs Song::mod_name; docs/rbm.md). The real-time player (task-043, crates/rb-player) builds a song with a sample set (Player::build / Player::load); it cannot change the mod of a song that plays, because how the original does it is not known: a new song object (msg 52 + reload, with the activation rules of docs/export_state.md section 3), a message that swaps the drum machines' samples in place, or something else, and what happens to the playing voices, the position and the process globals.

Do: find in Rebirth.exe the Mods menu handler (command 403 and the per-mod items) and what it sends to the DLL; read the DLL side; write the facts with addresses into docs/rbm.md; then make rb-player apply a mod as the original does (a Command, or Player::load of the rebuilt song, whichever the original's behavior is), with a test; the callback must stay allocation-free (crates/rb-player/tests/no_alloc.rs).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 docs/rbm.md says, with addresses, what the Mods menu sends and what the DLL does with a playing song
- [x] #2 rb-player applies a mod as the original does, with a test; the callback stays allocation-free
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. docs/rbm.md: a new section 14 (the Mods menu, the dialog, sub_42D80D/sub_426FB3, song msg 19 -> top-level 78 -> sub_10011D84) with addresses; cross-refs in docs/ui_spec.md (the Mods menu rows) and docs/export_state.md §8 [G] (the 'Mods menu' sender is resolved).
2. rb-engine: ChannelDevice::set_samples (808: slot 6 then the table; 909 the same; placeholders nothing), Engine::set_samples (sub_10011D84: 808 then 909). What the swap lets go of (the old tables, the samples the silenced voices still hold) goes into a fixed-capacity samples::Retired, so the callback frees nothing.
3. rb-player: Command::Mod(Box<ModSwap>) (song msg 19), Player::set_mod, Garbage::Mod; addressed to the newest song like every command.
4. Tests: engine unit test (voices silenced, table replaced, retired filled); player tests (a mod set on a fresh stopped song equals the song built with it; a live switch keeps position and play and changes only the drums; switching back); no_alloc covers the command.
5. cargo test, clippy, fmt; corpus golden (no engine output change expected).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-aaf083ecbcd73284f

RE (IDA copies in scratchpad/task-056, headless, closed save=false):
- Exe, the commands: 403 (Select Mod...) is handled by the song window's command handler sub_42C204 (0x42C294 -> sub_41A482: CModSelectorWindow, ctor sub_419460, vtable 0x4BB69C, command handler sub_41A0C5). The per-mod items are commands 5000+i (0x1388..0x144F) in the main frame's handler sub_40C163 (0x40C427..0x40C489): the active song window (sub_40B4A6), mod = sub_415578(i), sub_42D80D(window, mod), dword_4DA788 = mod.
- Dialog: a list selection already opens the mod (sub_416300 at 0x41A16A; ref-counted, word +1088; the open mod's ctor sub_41813E/sub_417AE9 sends top-level 55/56 through sub_41693C) and plays its startup sound (sub_426C67 at 0x41A180, msg 58 when +1088 == 1 and the preference is on). OK (0x41A2B6..0x41A305) calls sub_42D80D(window, mod) and closes the dialog.
- sub_42D80D: if the document's mod (sub_426F83) differs: sub_426FB3(doc, mod), then the window is detached (sub_42D7F7(0)), destroyed (sub_47350E) and rebuilt (sub_42DA72(doc, 0, rect)), which reloads the panel skin; its sub_40D163 at 0x42E5AE sends msg 53 only when the document is not already current (0x40D178), and it is.
- sub_426FB3(doc, mod): modified flag (sub_42590B), doc+4 = open the new mod (sub_416300: msg 55/56 if not loaded), sub_4259FF(id) = SONG msg 19 {u16 BE mod id} (push 13h at 0x425A31, sender sub_40B1E3, reply wait sub_40ABE5), release the old open mod (sub_4165BE: one-deep cache, the cached one deleted by sub_4162BC; the file mod dtor sub_418A87 sends top-level 57 {id} via sub_416EF9 at 0x418EA9 -> DLL sub_1001DB60 frees it), then sub_426C67 (startup sound).
- DLL: song handler sub_10011ED0, table 0x10151D6C+4*msg: msg 19 -> 0x10011F67, which forwards the message as top-level 78 {slot, 19, id} (byte_100DC7B0 'N', slot +0x178) to the EngineIdle queue (sub_1001D690 -> sub_1001D694). EngineIdle dispatch sub_1001D6D4 (table 0x101531F8+4*msg): 78 -> 0x1001D7B9 -> sub_10012940(song engine+0x14+4*slot), case 19 at 0x1001296D: id = be16, mod = sub_1001A860(id) (the DLL's mod list dword_10118280 by id), if found sub_10011D84(song, mod), then reply 39.
- sub_10011D84 (song lock +0x154 held, sub_10009480/sub_100094A0): song+0x174 = mod (0x10011DA6); the device of type 3 (808) gets sub_10028290: vtable slot 6 ([eax+18h] at 0x10028297, the voices silenced) then +0x18D8 = mod (0x100282A0); the device of type 4 (909) gets sub_1002B8C0: slot 6 then +0x18DC = mod (0x1002B8D0). Nothing else: no new song, no msg 52/53, the transport, position, play state, 303s, effects, mixer and the delay line (only the delay's slot 6 clears it) are untouched.
- [G] not modelled: with other song windows open, destroying the window may MDI-activate another one (sub_42D7BE -> msg 53) and the rebuilt window activates this one again. rb has one song.

Implementation:
- rb-engine: samples::Retired (fixed room: 2 tables, 32 samples); drums: Voice::retire and each voice class's retire; ChannelDevice::set_samples (default no-op), Tr808/Tr909 = slot 6 (silence), retire the voices' samples, replace the table, keep the old one; Engine::set_samples (808 then 909, as sub_10011D84).
- rb-player: Command::Mod(Box<ModSwap>), Player::set_mod, Garbage::Mod; applied to the newest song (target()); the box returns through the garbage queue.
- Why Retired: the voices keep an Arc<Sample> after slot 6 (as the original keeps its pointer); once the old table is dropped on the UI thread, a voice retriggered with the new kit would free the last ref in the callback. Shown: with Voice::retire disabled, no_alloc's new check (two changes, first kit collected on the UI side, then 40 armed calls) fails with 'the callback allocated or freed 46 times'; restored, it passes.
- Failed first try: unit tests stepped at tick 24 (same 16th as tick 0, no retrigger); steps are 48 ticks apart.
- Tests: rb-engine tr808/tr909 set_samples_silences_then_replaces_the_table; rb-player tests/mod_change.rs (a_mod_set_on_a_stopped_song_equals_building_with_it: full_mix, change to a half-scale kit before Play == built with it, 80 calls, and != the built-in kit; a_mod_change_on_a_playing_song_keeps_its_place: drums_kit, after 40 calls a silent kit, 40 more calls tick for tick equal to an unchanged twin, output peak 0 from the applying call on, generation 1, then the built-in kit sounds again); no_alloc.rs extended.
- Results: cargo test 350 pass (after rebase on 0543203); clippy -D warnings clean; corpus_golden release 4/4; rb compare --manifest with RB_MOD_DIR (recrypted) and target/Standard.rbm: 175 of 175 equal (no engine output change).
- Docs: docs/rbm.md section 14 (+ summary bullet), docs/ui_spec.md Mods rows, docs/song_load.md §2 step 2 [G] resolved (sub_41813E is built by sub_416300 for a file mod's first user). No defect of the original found.
- Not modelled: the startup sound (msg 58, preview player) in rb-player; the multi-window MDI msg 53 round trip [G].

Review round 1 (send back): (1) filed the follow-ups: task-074 (rb-player: the startup sound, msg 58, preview player), task-075 (RE: the multi-window msg 53 round trip [G], low), task-076 (check msg 19 against the DLL with rbhost --live, @agent-complex); all m-3, depend on task-056; docs/rbm.md 14.2/14.3 point to them. (2) docs/export_state.md section 8: the '[G: the Mods menu]' senders replaced by sub_41A0C5 (0x41A180) and sub_426FB3 (docs/rbm.md 14.1); sub_42A799 is the logo click, sub_42A80E left [G]. (3) The GLOB save claim checked in the disassembly: the save sub_4270F7 copies the document's open mod name (mov ecx,[ecx+4]; call sub_41691A at 0x427522..0x42752B; sub_41691A = strcpy from +1595, 0x416936) and passes it to the RBS writer sub_438025 (0x42758A), which writes GLOB (0x43817C); the GLOB field offset is left [G]. Also the dialog's preselection is now [C]: 0x4197A8..0x4197D0, unknown_libname_8 at 0x426F72 = mov eax,[ecx+4]. Optional two-Mod-boxes no_alloc case not added: Player::send collects the garbage before every push, so two Mod boxes cannot be in the queue at once through the public API (GARBAGE_PER_COMMAND is per command, one box per Mod).

Review round 2 (merge except one address): sub_41691A's +1595 (add eax, 63Bh) is at 0x416924, not 0x416936 (the epilogue; that address was in the round-1 note, the doc had none). docs/rbm.md 14.1 dialog and save bullets now cite 0x416924.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The Mods menu (403 Select Mod... via CModSelectorWindow sub_41A0C5 OK; items 5000+i via sub_40C163) calls sub_42D80D -> sub_426FB3, which opens the new mod (msg 55/56 if needed), sends song msg 19 {u16 id} (sub_4259FF, 0x425A31), releases the old mod (later msg 57) and plays the startup sound (msg 58); the song window is rebuilt for the new skin, no msg 52/53. The DLL defers msg 19 (0x10011F67 -> top-level 78 -> sub_10012940) to sub_10011D84: song+0x174 = mod, the 808 and the 909 each get slot 6 then their sample table (+0x18D8 / +0x18DC; sub_10028290, sub_1002B8C0). Nothing else changes: the song plays on. Documented in docs/rbm.md section 14.
rb: Engine::set_samples / ChannelDevice::set_samples (rb-engine) and Command::Mod / Player::set_mod (rb-player); the old tables and the voices' samples go back in the command's box (samples::Retired), so the callback frees nothing.
Branch worktree-agent-aaf083ecbcd73284f, one commit. cargo test 350 pass, clippy clean, corpus_golden 4/4, rb compare 175 of 175 equal (engine output unchanged).
<!-- SECTION:FINAL_SUMMARY:END -->
