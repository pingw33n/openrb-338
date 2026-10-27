---
id: TASK-076
title: Check song msg 19 (the mod change) against the original DLL with rbhost --live
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-27 14:33'
updated_date: '2026-10-27 15:24'
labels:
  - tests
  - rbm
  - engine
milestone: m-3
dependencies:
  - TASK-056
references:
  - docs/rbm.md
  - tools/rbhost/README.md
  - crates/rb-player/tests/live_messages.rs
  - crates/rb-player/tests/mod_change.rs
priority: medium
type: task
ordinal: 100000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: task-056 modelled the mod change from the disassembly (docs/rbm.md section 14: song msg 19 {u16 BE id} is forwarded to the idle loop as top-level 78 {slot, 19, id_hi, id_lo}; sub_10011D84 runs slot 6 on the 808 and the 909, then replaces their sample table +0x18D8 / +0x18DC) and checked it only with rb's own tests. It was never run against the DLL.
Do: extend the live-session check of task-055 (crates/rb-player/tests/live_messages.rs writes an RBLV file of msg-78 records; tools/rbhost rbhost.exe --live SESSION.rblv SONG.rbhm OUT.wav renders it in the original DLL; RB_LIVE_OUT=DIR compares) with a mod-change record mid-song: a drum song playing with one mod loaded (msg 55 via rbhost --mod or a second loaded mod), then at call K a 78 {slot, 19, id} record switching to another loaded mod (both must be loaded with msg 55 first, the built-in kit is id 411 via msg 56), compared byte for byte with rb-player sending Command::Mod at the same call. Use recrypted mods (RB_MOD_DIR) and the standard mod; rbhost runs take one redc-win-run permit (doc-001). Add rbhost support for loading two mods if it lacks it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 an rbhost --live render with a msg-19 record mid-song equals rb-player's render with Command::Mod at the same call, byte for byte, for at least one 808 and one 909 song
- [x] #2 the comparison is a test (env-gated like live_messages.rs) and the result is recorded in docs/rbm.md section 14
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. rbhost: a repeatable --load-msg FILE option that sends a top-level 55 (file mod) or 56 (built-in, {411}) message file after the song mod's message and before the songs are created (reply 'C'), so two or more mods are loaded and a live song msg 19 can switch between them.
2. live_messages.rs: generalize render() to a song, a starting kit and a session of commands or mod changes; a mod change goes to the .rblv as {78, slot, 19, id BE} and to the player as Player::set_mod with the same mod's samples (recrypted .rbm from RB_MOD_DIR via rbm::Rbm::load_samples over the DLL's built-in kit). New tests, skipped without RB_MOD_DIR: drums_kit (808 + 909) built-in -> PBE2 -> Orbit2 -> built-in while playing; an 808 song and a 909 song started with a file mod and switched to another / to the built-in kit.
3. Run rbhost --live for each session on the test machine (one permit, C:\re\rb\wt076, deleted after), record the hashes as constants, compare. If rb differs, find the cause in the disassembly (sub_10011D84, sub_10028290, sub_1002B8C0, the voices' slot 6) and fix rb-engine/rb-player.
4. docs/rbm.md section 14 and tools/rbhost/README.md: the result and the option. Corpus: rb compare 164/164 and 175/175 with RB_MOD_DIR.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-aa82baeeec2caad06

rbhost: new repeatable --load-msg FILE (tools/rbhost/rbhost.c main, after the song mod's 55/56): sends a message 55 (0x2547 bytes) or 56 {u16 id} (3 bytes) and expects reply 'C' (67); the songs keep the song mod's id. Build sha256 cc36309b3c2c10810ac0ca4a695a85ff3c0e6d27f66a603eebe7088263256a60 (tools/rbhost/build.sh), same hash on the machine.
live_messages.rs: render() now takes the song, the starting kit, the call count and events Ev::Cmd(Command) | Ev::Mod(id, samples); Ev::Mod goes to the .rblv as {78, 0, 19, id_hi, id_lo} (sub_4259FF's song msg 19) and to the player as Player::set_mod before the same call. Mod samples: rbm::Rbm::parse + KeyTable::embedded + load_samples over the DLL's built-in kit (as rb render's load_mod). Three sessions, 400 calls, idle 0, pattern mode + play at 0: mod_kit = drums_kit built with 411, PBE2 412 at 100, Orbit2 413 at 200, stop 270, 411 at 290 (stopped), play 300; mod_808 = 808_cymbal built with Metallicon 412, MSM2 413 at 150, 411 at 250; mod_909 = 909_crash built with Orbit2 412, 411 at 150, PBE2 413 at 250.
Machine run (2026-10-27): scratchpad wt076/prep.py (rbsmsg.message_file(song, 412|None), rbm.mod_message for the original mods with RB20FUL.DAT, path C:\re\rb\wt076\MOD.rbm, builtin.msg = 38 01 9B hex... i.e. bytes 56,1,155), wt076/run.sh: locky acquire redc-win-run 1 (6 of 6 free before), upload to C:\re\rb\wt076, rbhost.exe --private-port [--mod-msg X_412.msg] --load-msg ... SONG.rbhm --live NAME.rblv NAME.wav, rcs hash, release on EXIT trap; folder deleted after (rcs rm -r rb/wt076). Logs: 'mod 412/413 loaded (msg 55)', 'mod 411 loaded (msg 56)', 'live session: 7|4|4 messages, 400 calls'; exit 0.
Hashes, rbhost = rb-player on the first run, no rb change needed: mod_kit 0ded17da6c345854237f3176b1c605ddc117defd3a331ba09a36f5cba3cbe841, mod_808 3954a36be3017e835f22bd15bfed4dfde4b90135862ac8b5ad317df28f4ea4ba, mod_909 cf414eb6493779e935fa8e12fea27c9fadeaf045beaaa5febb9722efbc98d0ba. Control: the same sessions without the mod changes hash otherwise in rb (asserted in the test).
Reproduce: RB_MOD_DIR=<recrypted mods> RB_LIVE_OUT=DIR cargo test --release -p rb-player --test live_messages; without RB_MOD_DIR the three mod tests return early (skipped).

Rerun with the final rbhost build (sha256 c45c098f8b9e6ef619c1e5aeae34ada1304ad8317da69aa99aa2366e5b03cb36; the first run's cc36309b... lacked only the lsize < 3 guard of --load-msg): the same three hashes. Permit released (6 of 6 free), C:\re\rb\wt076 deleted. No defect of the original found; no rb change needed.
Checks: cargo clippy --all-targets -D warnings clean; cargo test (RB_MOD_DIR set) 387 pass; rb compare --manifest tests/refs/manifest.json --standard-mod target/Standard.rbm 164 of 164 equal, with RB_MOD_DIR 175 of 175; tools/rbhost python tests 43 OK.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Song msg 19 (the Mods menu's mod change of an open song) is checked byte for byte against ReBirth.dll 2.01: three rbhost --live sessions with msg-19 records mid-song (drums_kit with 808 and 909: built-in -> PBE2 -> Orbit2 while playing, -> built-in while stopped; 808_cymbal: Metallicon -> MSM2 -> built-in; 909_crash: Orbit2 -> built-in -> PBE2) give the same WAV sha256 as rb-player with Player::set_mod at the same calls, on the first run, with no change to rb. A control asserts the sessions without the changes render otherwise.
Changes: tools/rbhost/rbhost.c gains the repeatable --load-msg FILE (one more mod, msg 55 or 56, before the songs are created; README updated); crates/rb-player/tests/live_messages.rs generalizes render() to a song, a kit, a call count and mod-change events and adds the three env-gated tests (RB_MOD_DIR; rbm as a dev-dependency of rb-player); docs/rbm.md 14.4 records the sessions and hashes, and the [C] note of section 14 now says it was run in the DLL.
Branch worktree-agent-aa82baeeec2caad06, one commit. Tests: cargo test 387 pass, clippy clean, corpus 164/164 and 175/175 with RB_MOD_DIR, rbhost python tests 43 OK.
<!-- SECTION:FINAL_SUMMARY:END -->
