---
id: TASK-047
title: 'UI menu: load/save song, export, choose RBM, Vintage flag'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-26 16:23'
updated_date: '2026-10-27 15:42'
labels:
  - ui
milestone: m-3
dependencies:
  - TASK-045
  - TASK-046
  - TASK-028
  - TASK-033
  - TASK-056
priority: medium
type: feature
ordinal: 47000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The menu, reduced to what the user asked for (2026-10-26): Load song, Save song (and Save as), Export (the song loop to .wav/.flac/.ogg through the m-1 export path, so the file equals `rb render`'s), Choose RBM (the mods found in the mods folder or a file picked in the in-app browser; "Standard ReBirth" for the built-in kit), and the Vintage ReBirth Synth Sound flag (GLOB byte 482, docs/song_load.md section 3). Unsaved-changes prompt inside the app. Nothing else from the original menu.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Load, Save and Save as round-trip a song (the saved file re-opens identically)
- [x] #2 Export writes the same file as rb render for the same song and options
- [x] #3 Choose RBM switches the kit, including back to the standard kit; the Vintage flag toggles the sound model and is saved with the song
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. rb-cli: split a library (rb_cli: encode, mods, render) from the bin so the UI reuses rb render's code for the export (Renderer::render_to_file).
2. rbs: Song::empty() for a new song.
3. rb-ui: document.rs (compose: the file as read + the panel/link state), menu.rs (File, Mods, Options, prompt, export thread), Panel::value/vintage, Link::set_mod and msg 35 id 21, main.rs.
4. Tests (tests/menu.rs): corpus save identity, edits saved, export vs rb render, mod and vintage, prompt. Docs: ui_spec 18, rbm 14.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a421bbda8f579afdf

Design and evidence.
- Save = rbs::Song as read + panel/link state (document::compose). The original saves the live device blocks (sub_43920E.. -> sub_438025). Controller c of a device is parameter c; patterns from Link::patterns. A 4.2 file keeps the file byte where the panel still shows what the load made (the 909 mid tom decay copy of the load, docs/song_load.md 4.8); older files are saved as 4.2 with the converted values and converted track events (the 808 tom tunings).
- Corpus check (tests/menu.rs saving_an_unedited_corpus_song_gives_back_the_file): 163 songs (diag + shipped); 146 of 4.2 are byte-identical after open + save; the 17 shipped 4.0/4.1 songs load to the same engine song after save. First run failed 56 songs on tr909.params[16] (the load copies LT decay into MT decay) -> baseline keep; 17 older songs on track events (tom tune conversion) -> converted events written.
- Two latent model bugs found by the save and fixed: Panel::adopt never took the engine value of a mixer channel distortion insert (no toggle entry until pressed) nor of a pattern section on switch (the toggles branch came first). A 4.1 shipped song showed 303 on although the file has it off.
- Export: rb-cli is now lib + bin (rb_cli::render::Renderer::render_to_file, same encoders); the UI composes the song, writes bytes, parses and converts again, and renders on a thread. tests/menu.rs export_writes_the_file_rb_render_writes compares the UI export with Renderer on the saved file for 303_saw_lowcut and Calm in wav, flac and ogg: wav and flac byte-equal, ogg equal after zeroing the random stream serial and checksum of every page (the encoder picks a random serial per file; rb render itself is not byte-reproducible for .ogg). Export is disabled in pattern mode as in the original (spec 12); rb render only warns there.
- Mod: Opening a song looks its GLOB mod up in the Mods folder (settings, RB_MOD_DIR, rb render defaults); not found or not recrypted -> built-in kit, notice, name kept. A choice: Link::set_mod (forward pending events, Player::set_mod, song msg 19), mark modified, name and INFO URLs for the save. a_mod_changes_the_kit_and_is_saved (RB_MOD_DIR=recrypted-mods, PBE2 on drums_kit.rbs): the output differs from the standard kit, equals a song opened with its own mod, a save+reopen with Standard equals the standard output, the export with the mod equals rb render with --mod.
- Vintage: Panel::vintage, Event song.vintage -> msg 35 id 21 -> Command::Vintage; tests check the engine frame.vintage, the save (GLOB byte 482) and the reopen.
- Options menu: I included the keyboard mode (Program Synth Ctrl+F / Select Patterns Ctrl+G, App::key_mode) as task-070 asked; the Vintage item is in Options (the original has it in Edit).
- Unsaved prompt: after the browser chose a file (Open) and on the window close (CancelClose); S/Enter save, D/N do not, Esc cancels; Save of a song with no file goes through Save As and then continues.
- The menu bar is 22 points above the panel (App::menu, set by rb-ui; the 1:1 headless panel tests run without it). Screenshot of rb-ui on Calm.rbs checked: bar renders, DIST lit.
- Not done: the mod panel art (original redraws the window with the mod bitmaps; rb draws SVG) -> Decide task-078; the mod startup sound (task-074); Record has no effect (task-057) so no track edits are saved. Not confirmed: the original marks a song modified for which edits (assumed: every edit the save writes, not play/stop/locate).
- Gated tests: RB_STANDARD_MOD=/Users/pingw33n/dev/rb/target/Standard.rbm, RB_MOD_DIR=<scratchpad>/recrypted-mods; without them the export and mod tests skip.
- Verification: cargo fmt --check, clippy --all-targets -D warnings clean; cargo test --release (workspace, after the rebase onto 69430ae): 406 passed, 0 failed.

Review round 1 (2026-10-27): (1) rb-cli encode.rs ogg now uses VorbisEncoderBuilder::new_with_serial(.., 0x7262): .ogg output is deterministic; the notes' earlier line 'rb render is not byte-reproducible for .ogg' no longer holds. tests/menu.rs compares the Ogg bytes exactly; no committed reference is .ogg (tests/refs/manifest.json has none); rb-cli render tests pass. (2) app.rs browse: Poll::Cancelled clears after_save; test a_cancelled_save_as_from_the_prompt_ends_the_action. (3) regression test the_panel_takes_the_distortion_insert_and_the_section_switch_from_the_song (mixer_dist_switch.rbs, A Taste of Haste.rbs). (4) ui_spec 18: the 4.2 version write is marked [G] (sub_438025 not read). (5) task-078 decided by the user: keep SVG art; docs updated. Workspace cargo test --release with the env after rebase onto 9a32b9f: see the final run.

Review round 2 (2026-10-27): commit message amended only (Result: Ogg exact with fixed serial; Testing: 411 pass with the env; Decision: task-078).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The menu is done: File (Open Ctrl+O, Save Ctrl+S, Save As, Export loop to wav/flac/ogg), Mods (Standard ReBirth, the Mods folder, Choose file), Options (Vintage ReBirth Synth Sound, keyboard mode), and an unsaved-changes prompt on open and close. Save composes the file as read with the panel state over it (146 corpus songs byte-identical, 17 older ones load identically); Export runs rb render code (rb-cli is now lib + bin) and equals its file (ogg modulo the random stream serial); a mod change is song msg 19 via Link::set_mod and a save writes the name; Vintage is msg 35 id 21 and GLOB byte 482. Two model bugs fixed on the way (distortion insert and pattern on switch never followed the engine). Branch worktree-agent-a421bbda8f579afdf, one commit. cargo test --release: 406 passed. Follow-up: Decide task-078 (mod panel art).
<!-- SECTION:FINAL_SUMMARY:END -->
