---
id: TASK-074
title: >-
  rb-player: the mod's startup sound on a mod change (msg 58, the preview
  player)
status: Done
assignee: []
created_date: '2026-10-27 14:33'
updated_date: '2026-10-27 20:01'
labels:
  - engine
  - rbm
  - ui
milestone: m-3
dependencies:
  - TASK-056
references:
  - docs/rbm.md
  - docs/export_state.md
  - crates/rb-player/src/lib.rs
priority: low
type: feature
ordinal: 98000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: when the Mods menu changes the mod of an open song, the original plays the new mod's startup sample: sub_426FB3 ends with sub_426C67, which sends top-level msg 58 {u16 mod} when the mod record's word +1088 is 1 (the mod just opened) and the preference 'Play Mod Startup Sound' (app byte +0x533, sub_4098C3) is on; the Mod Selector dialog does the same when an entry is selected (sub_41A0C5, 0x41A180). The DLL's preview player (docs/export_state.md section 8: SafeSoundCallback mixes the mono int16 sample at engine +0x428 from +0x42C after the song, h = s/2 rounded toward zero, L and R saturated; cleared at the sample end and by msg 57) is not modelled in rb-player (task-056 left it out; docs/rbm.md section 14.3). The startup sample is slot 46 of the mod (Startup.aif, resource 1024 for the built-in kit).
Do: add the preview player to rb-player (a Command that starts the slot-46 sample of a sample set, mixed after the song as section 8 describes, allocation-free in the callback), and let the UI send it on a mod change and a logo click (sub_42A799) when the preference is on. A Decide: task is needed only if the preference's default or storage is not obvious from docs/test_machine.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 rb-player mixes a startup sample after the song as docs/export_state.md section 8 describes, with a unit test of the rounding and saturation
- [x] #2 the callback stays allocation-free (crates/rb-player/tests/no_alloc.rs covers the new command)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. RE check (DLL and exe, disassembly): SafeSoundCallback's preview mix 0x1001AF15..0x1001B037, msg 58 sub_1001DC14, msg 57 sub_1001DB60, the mod record's +184 (sub_1001A030), CWaveSample's length +0x4C (sub_1000A9C0); exe sub_426C67, sub_42A799, the preference's default (prefs ctor) and storage.
2. rb-player: Command::Preview(Box<Preview>) (top-level msg 58: the startup sample, slot 46 / ID 1024, of a SampleSet, from position 0) and Command::PreviewStop (msg 57's clear); Player::startup_sound(&SampleSet). The callback keeps the preview's box (the replaced one goes back through the garbage queue) and mixes it after the song in each 1024-frame call: h = s/2 toward zero, L and R saturated, min(len - pos, n) frames, cleared at the end. The mix is a pure function with a unit test of the rounding and saturation.
3. Tests: a player test (the preview over a silent song, across calls, the end, restart, stop, mixed after a playing song); no_alloc.rs sends the new commands (including a replace that hands a box back).
4. rb-ui: Link keeps the current song's samples (load, set_mod) and sends the startup sound; App::use_kit sends it after a mod change when the setting is on (sub_426FB3 -> sub_426C67; the mod's user count is 1 for rb's one song); a logo click (panel rect of sub_42A799) sends it as the original (no preference test there). Settings gets startup_sound (default on, as the prefs ctor), stored in ui.conf, with an Options menu checkbox.
5. Docs: export_state.md section 8, rbm.md 14.3, ui_spec.md (the logo, the mod change, the preference); notes; fmt, clippy, cargo test --release.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a2a1ba240e2570321

RE (disassembly, ReBirth.dll.i64 / Rebirth.exe.i64 headless copies, closed unsaved):
- Mix, SafeSoundCallback 0x1001AF15..0x1001B037 [C]: sample = engine+0x428, pos = +0x42C; count = sub_10004630(len - pos, n) (a signed min, 0x10004638); len = CWaveSample+0x4C (sub_1000AA80), data = +0x50 (sub_1000AA60). Per frame: esi = s; add esi,80000000h; adc esi,80000000h; sar esi,1 (0x1001AF7E..0x1001AF8A) = (s + (s<0)) >> 1 = s/2 toward zero; shl/sar 10h (no-op, h fits); L = sat16(L+h), R = sat16(R+h) (cmp 7FFFh / FFFF8000h, 0x1001AFA5..0x1001AFE9). After: pos += count stored (0x1001B037); pos == len clears +0x428 (0x1001B011..0x1001B022). Runs after the song or the zeroed buffer, whatever the try-lock did.
- CWaveSample ctor sub_1000A9C0: +0x4C = the frame count (0x1000A9DC), buffer 2*len + 0x400 bytes (the 512 padding samples), so len excludes the padding = rb's Sample::frames().
- Msg 58 sub_1001DC14: mod = sub_1001A860(BE id); none found: nothing; else +0x42C = 0 (0x1001DC3C), +0x428 = record+184 (0x1001DC48). A 58 while one plays restarts at 0 with the new sample.
- Record +184 = slot 46 (sub_1001A030: v4[46], 0x1001A178 for 411 = ID 1024; 0x1001A2EB for a file mod: its own when the mod has it (+732 flag, 0x1001A2CC), else ID 1024). rb's SampleSet::get(STARTUP_ID) is the same (a mod's set fills missing slots with the built-in ones).
- Msg 57 sub_1001DB60: engine+1064 (0x428) = 0 at 0x1001DBA4 unconditionally (whatever mod id), then frees the mod.
- Exe sub_426C67: movsx [mod+440h] (word +1088) == 1 (0x426C6E/0x426C75), then sub_4098C3 (app byte +0x533) at 0x426C85, then sub_40ABB5(mod id) = top-level 58 (sub_40B142(58, 2 bytes), 0x40ABDF).
- Logo click sub_42A799: hit rect sub_479B06(458,1,608,49), then sub_40CB32, doc mod id (sub_4166D0), sub_40ABB5 (0x42A7D3..0x42A7F1): NO preference test and no +1088 test. The logo always plays the mod's startup sound; only the mod-open paths (sub_426C67) honor the preference. rb-ui follows the disassembly here (the task text read 'logo click when the preference is on').
- Preference default: ON. The prefs block ctor sub_41CAA4 sets +536 = 1 (0x41CB13) (+536 is what sub_40DEF2 passes to sub_4098AA at 0x40E2F8), and the app ctor sub_40D1D2 sets +0x533 = 1 (0x40D338). Written by the dialog sub_40E34A (0x40E7AE) and sub_41E959. Storage in the original: the prefs file byte 0x20A (docs/test_machine.md section 5). Default and storage are obvious, so no Decide task: rb-ui stores it in its ui.conf (startup_sound=0|1, default 1), switched in the Options menu (rb has no Preferences dialog).

Implementation:
- crates/rb-player/src/lib.rs: Command::Preview(Box<Preview>) (msg 58) and Command::PreviewStop (msg 57's clear); Player::startup_sound(&SampleSet) (SampleSet::shared(STARTUP_ID), slot 46); Audio keeps the preview box, position and on flag; Audio::call mixes after the render and the fault fill with the pub fn mix_preview (min(len - pos, frames), s/2 toward zero via Rust's truncating division, L and R clamped to int16), clears at pos == len; a replaced box goes to Garbage::Preview. Audio::preview_position for tests.
- rb-ui: Link keeps the newest song's kit (new, load, set_mod) and Link::startup_sound queues Command::Preview behind its backlog; App::use_kit calls it after set_mod when Settings::startup_sound is on; App::focus_click calls it on a press in layout::on_logo (panel x 474..607, y 1..48, docs/ui_spec.md section 3) whatever the preference (sub_42A799 does not test it); Settings::startup_sound (default true, startup_sound=0|1 in ui.conf).
- Not done here, as decisions or follow-ups: a menu switch for the preference (the menu is the user's reduced menu; Decide task-086, recommendation an Options checkbox; I first added one and removed it); the startup sound of a song's open and the app start (sub_425C4D, sub_426CA8 also call sub_426C67): task-087.
- Failed attempt: the first rb-ui mod test ran 60 calls, but PBE2's Startup.aif is longer than 61440 frames; the test now runs len/1024 + 2 calls.
- Rebased onto main b5df776 (the ui-design-1 redesign): no conflicts; the logo is still at transport (458, 1) in rb-ui-art parts.rs.
Tests (RB_STANDARD_MOD=target/Standard.rbm, RB_MOD_DIR=recrypted mods): cargo test --release: 440 passed, 0 failed (corpus_golden included: every corpus entry still EQUAL, the offline render path is unchanged: rb render and the export send no msg 58); clippy -D warnings clean; fmt clean. New: rb-player lib tests the_half_rounds_toward_zero_as_the_original (all 65536 int16 values against a model of add/adc/sar), left_and_right_saturate_each, it_mixes_what_is_left_and_no_more; tests/preview.rs (built-in startup sound over the empty song and over full_mix playing = base + mix call by call, cleared at the end; restart with a new sample, the old box returned, PreviewStop); no_alloc.rs (startup sound over the playing song, a replacement whose old box held a sample only it owned, the stop, a sample played to its end: 0 allocator events); rb-ui menu.rs a_mod_change_plays_the_startup_sound_when_the_preference_is_on (PBE2: on = off + PBE2's Startup.aif mixed), the_logo_plays_the_startup_sound (with the preference off); layout the_logo; settings round trip.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
rb-player has the DLL's preview player: Command::Preview (top-level msg 58, built by Player::startup_sound from a SampleSet's slot-46 Startup.aif sample) and Command::PreviewStop (msg 57's clear), mixed into each 1024-frame call after the song by rb_player::mix_preview exactly as SafeSoundCallback 0x1001AF15..0x1001B037 (h = s/2 toward zero, L and R saturated, min(len - pos, n) frames, cleared at the end), allocation-free (the replaced box goes back through the garbage queue). rb-ui plays the new mod's startup sound after a Mods-menu change when Settings::startup_sound is on (default on, as the original's prefs ctor sub_41CAA4 0x41CB13; stored as startup_sound=0|1 in ui.conf), and on a logo click whatever the preference, because sub_42A799 sends msg 58 without testing it (disassembly 0x42A7D3..0x42A7F1). Docs: export_state.md section 8 (details and rb), rbm.md 14.3, ui_spec.md sections 3 and 18.
Branch worktree-agent-a2a1ba240e2570321, one commit on main b5df776. Tests: cargo test --release 440 pass (corpus_golden EQUAL), clippy and fmt clean.
Follow-ups: Decide task-086 (a menu switch for the preference; recommended: an Options checkbox), task-087 (the startup sound on a song's open and at start, sub_425C4D / sub_426CA8).
<!-- SECTION:FINAL_SUMMARY:END -->
