---
id: TASK-155
title: >-
  AIFF reader: ignore junk after COMM and SSND, as the original does (4 mods
  fail to recrypt)
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-30 09:00'
labels:
  - engine
  - rbm
  - re
milestone: m-5
dependencies: []
references:
  - crates/orb-engine/src/samples/aiff.rs
  - docs/rbm.md
  - docs/initial_investigation.md
  - docs/original_bugs.md
priority: high
type: bug
ordinal: 173000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: on 2026-10-30 the user recrypted a folder of 317 mods (~/rb-stuff-peff/ReBirth Mods, the user's, read only), using `orb mod recrypt --iso CD.iso DIR -o OUT`. Four failed with "the mod's sample X: the <junk> chunk runs past the end of the file":
- Black_Steel.rbm (Startup.aif);
- Black_Steel_2.rbm (TR808MT.aif);
- O.S.C.rbm (Startup.AIF);
- mellow.rbm (Startup.aif).

The supervisor decrypted them with tools/rbm.py extract and walked the chunks:
- The first three have FORM, COMM and SSND, and the SSND size is 8 too small: it leaves out SSND's 8-byte offset and block-size header. For Black_Steel, COMM frames are 226799, so the audio is 453598 bytes = the SSND size; it ends at the file's end, 453652. The FORM size covers the file, so 8 bytes remain after the SSND size and read as a chunk header with a huge length.
- mellow has COMM, INST (20) and SSND, with a correct size, then 20 stray bytes inside the FORM.
- The audio itself is complete in all four.

crates/orb-engine/src/samples/aiff.rs `parse` walks every chunk to the end of the FORM and fails on a chunk that runs past the file (about line 52), even after COMM and SSND were found. The original's chunk walk `sub_10004F94` (docs/initial_investigation.md, AIFF parsing, task-020) is believed to look for one id at a time and stop at the first match, so it never reads the trailing bytes. That is a guess; confirm it.

What:
1. RE: in ReBirth.dll, check `sub_10004F94` and its callers from `sub_10003300` and `sub_10002FC4`, against the disassembly, not only Hex-Rays:
   - Is each chunk searched for from the start (or from the current position)?
   - Does the walk stop at the first match?
   - What happens at a chunk whose size runs past the stream: a failure, or ignored?
   - What happens to a damaged chunk before SSND?
   Record it, with addresses, in docs/rbm.md 11.2 (or a new 11.x next to the stale-SSND case of task-058), and update the [G] in docs/initial_investigation.md. If the original has a defect here, add it to docs/original_bugs.md.
2. Fix aiff.rs to the original's rule. Probably: stop the walk once COMM and SSND are both found, or search each id as the DLL does, so trailing bytes are never read.
3. Tests, with synthetic AIFFs built in the test, not the user's files:
   - SSND size 8 short with 8 trailing bytes;
   - 20 stray bytes after a correct SSND;
   - a junk chunk before SSND, behaving as the DLL does.
   Every existing sample test still passes.
4. Verify on the user's mods, read only:
   - recrypt the four into a scratch folder with `orb mod recrypt --iso "$ORB_CD_ISO"`; all succeed;
   - `orb render` a song with one of them; if the folder has no song, use any shipped song with --mod;
   - re-run the whole folder: 317 recrypted, 0 failed.
   Don't commit or copy the mods.
5. The engine change is checked against the corpus (175/175) and the gated thirdparty test (ORB_THIRDPARTY_MODS_DIR=~/rb-mods/mods, ORB_THIRDPARTY_SONGS_DIR=~/rb-songs/songs). Both must be unchanged.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The original's chunk-walk rule is confirmed in the disassembly, with addresses, and recorded in docs/rbm.md and docs/initial_investigation.md (docs/original_bugs.md if it is a defect)
- [ ] #2 aiff.rs follows that rule; synthetic tests cover a short SSND size with trailing bytes, stray bytes after SSND, and a junk chunk before SSND
- [ ] #3 The four mods recrypt, and the user's 317-mod folder gives 0 failures; a song renders with one of them
- [ ] #4 fmt, clippy and cargo test --release pass; the corpus is 175/175; the gated thirdparty test passes
<!-- AC:END -->
