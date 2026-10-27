---
id: TASK-031
title: Mod references in the corpus
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-27 06:08'
labels:
  - rbm
  - tests
  - testmachine
milestone: m-2
dependencies:
  - TASK-014
  - TASK-030
  - TASK-038
priority: medium
type: task
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Collect real RBM mods (from the test machine's installation or from the user; ask through a Decide task if none are available) and add mod songs to the corpus: songs naming each mod in GLOB, rendered by the decided capture protocol (rbhost with the msg 55 bank load, or the app's export), stored and listed in the corpus MANIFEST.md like the rest.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 At least three mods with songs using them are in the corpus with their references and manifest entries
- [x] #2 Re-capturing one mod reference equals the committed one
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. App session: rb_export.py RSI, Fonda Workout, Metallicon City (T=2, restore-prf). 2. fit.py --all with --mod per mod. 3. rbsmsg/run.py/fit.py: accept a song that names another mod when --mod is given (mod id override, name check). 4. rbcorpus.py: sets mod (rbhost, hash only) and modapp, mod record, env vars. 5. Capture, recapture, golden test update, docs/rbm.md.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-26: per the user, app-export validation is optimized first (task-038) before this task captures app exports again.

Branch: worktree-agent-aea307897f6317c15

Evidence (2026-10-27):
- Mods: PBE2.rbm (Pitch Black Edition 2.0, sha256 797ea0f7..., 3623024 B), MSM2.rbm (Massen's MSM 2.0, aced273b..., 2183516 B), Metallicon.rbm (Metallicon+, ef68879d..., 3532098 B). Orbit2 has no shipped song. Mod id 412 for rbhost.
- rbhost refs at N=9088 for the 8 mod songs: set 'mod' of tests/refs/manifest.json, hash + frames only. All non-silent (sampled check).
- App cross-check: one app session (rb_export.py --restore-prf --startup-sound off, 3 songs, ~15 s each incl. 22M-frame Metallicon City). fit.py --all (200 candidates each) with --mod and rbm.py's key table: unique N = 3536 (RSI, PBE2), 3536 (Fonda Workout, MSM2), 3504 (Metallicon City). rbhost equals the app WAV byte for byte (SHA-256), so rbm.py's table = the app's. N-N_t = -28,-40,-5 (unusual vs +48; not investigated).
- Size: RSI and Fonda Workout app FLACs are committed (tests/refs/modapp/). Metallicon City FLAC is 45.5 MB; tests/refs is 14 MB, so 59 MB > ~50 MB budget (task-006): hash only (sha256, frames, N) in modapp.
- Tool changes needed: rbsmsg.message_file(song, mod_override) and run.py/fit.py pass --mod-id and check the mod's INFO name against the song's (before, a song with another mod was rejected, so task-030's --mod only worked with a Standard song). rbcorpus.py: sets mod/modapp, --mod-dir/$RB_MOD_DIR, --rb20ful/$RB_RB20FUL, mod record (name, file, size, sha256, id), capture-app --set modapp --hash-only, repeatable --fit-dir reading fit.json.
- tool_commit in the entries is dc6467d (last commit touching tools/rbhost before this one; this commit changes rbsmsg/run/fit but not rbhost.exe: rbhost_sha256 8140febd949d is the exe hash).
- AC2: 'rbcorpus.py recapture mod' re-rendered all 8 and 'recapture modapp' the 3: all equal (sha256+frames; rbcmp exit 0 against the FLAC for RSI and Fonda Workout); check-flac: 158 FLACs decode to their WAV sha256.
- Golden test: mod/modapp sets are not run by corpus_golden.rs (it checks diag, app, shipped only; rb compare's sets are fixed too); the out-of-corpus test now also asserts each modded song is in 'mod'. Engine mod support is task-033.
- Tests: cargo test --release all pass; python unittest tools (27 rbhost + tools) OK.

Review round 1 (supervisor): all five fixed in the amended commit. (1) Supersedes the FLAC notes above: modapp is hash only (sha256, frames, N) for all three, the two FLACs and their manifest fields are removed (supervisor decision under task-006). (2) fit.py checks every song's mod name (run.check_mod_name) before anything is uploaded. (3) mod and modapp entries carry tool_change: task-031 next to tool_commit dc6467d, explained in MANIFEST.md and rbcorpus.py. (4) New tools/rbhost/run_test.py (exact name accepted; case/whitespace/other-mod/no-INFO refused; upload_mod refuses a mismatch), rbsmsg_test override tests (override id, no override keeps 411, non-Standard without override still refused); no unit test of fit.main itself (needs the machine), it calls the tested check_mod_name. (5) fit.json files committed in tests/refs/modapp/: RSI tried 200 matches [3536]; Fonda Workout tried 200 matches [3536]; Metallicon City tried 200 matches [3504]. Tests: cargo test --release pass, python tools 33 tests OK.

Review round 2: stale text fixed. Correction of the AC #2 evidence above: recapture equality is by hash (sha256 + frames) for all 8 mod and 3 modapp entries; there is no rbcmp against RSI/Fonda FLACs (modapp has none) and check-flac covers 156 FLACs (diag and app). MANIFEST.md and the commit message now say modapp is hash only (sha256, frames, N, fit.json files).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Eight mod songs (3 mods: PBE2, MSM2, Metallicon+) have rbhost references at N=9088 in tests/refs (set mod, hash + frames, mod name/file/size/sha256 recorded; the .rbm files and RB20FUL.DAT are not committed). Three app exports (RSI, Fonda Workout, Metallicon City) were fitted (N 3536, 3536, 3504, the only match among 200 candidates each; fit.json in tests/refs/modapp/) and equal rbhost with rbm.py's key table byte for byte: task-030's gap closed (docs/rbm.md section 8). modapp is hash only (sha256, frames, N). Re-capture of all mod entries equals the committed ones. tools/rbcorpus.py, rbsmsg.py, run.py, fit.py extended and tested; golden test skips the new sets; entries carry tool_change task-031. Branch worktree-agent-aea307897f6317c15, one commit. Tests: cargo test --release pass; python tool tests OK.
<!-- SECTION:FINAL_SUMMARY:END -->
