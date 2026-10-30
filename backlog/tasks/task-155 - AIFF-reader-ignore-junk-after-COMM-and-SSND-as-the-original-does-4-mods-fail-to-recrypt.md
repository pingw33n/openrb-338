---
id: TASK-155
title: >-
  AIFF reader: ignore junk after COMM and SSND, as the original does (4 mods
  fail to recrypt)
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-30 09:00'
updated_date: '2026-10-30 09:31'
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
- [x] #1 The original's chunk-walk rule is confirmed in the disassembly, with addresses, and recorded in docs/rbm.md and docs/initial_investigation.md (docs/original_bugs.md if it is a defect)
- [x] #2 aiff.rs follows that rule; synthetic tests cover a short SSND size with trailing bytes, stray bytes after SSND, and a junk chunk before SSND
- [x] #3 The four mods recrypt, and the user's 317-mod folder gives 0 failures; a song renders with one of them
- [x] #4 fmt, clippy and cargo test --release pass; the corpus is 175/175; the gated thirdparty test passes
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. RE (done before coding, see the notes): sub_10004F94, its callers 0x10003170 / 0x10002FC4 / 0x10003300, and the mod sample's memory stream (vtable 0x10030824).
2. aiff.rs: replace the single walk with a per-id search that follows sub_10004F94: the FORM check (id FORM, size >= 4 unsigned, the type ignored), the first header at 12 with no bound check, stop at the first match, next = pos + size + 8 (+1 odd) wrapping u32, not found when (next + 8) as i32 >= (FORM size + 8) as i32. A header or read the DLL would take outside the file is an error in orb (the DLL reads out of bounds); a walk that does not advance is an error (the DLL hangs). The SSND offset is skipped only when > 0 as i32.
3. Unit tests with synthetic AIFFs: SSND 8 short + 8 trailing bytes; 20 stray bytes after SSND; a junk chunk before SSND (size past the FORM end: missing SSND; landing outside the file: error; size -8: error); the FORM type ignored; the strict end bound.
4. Docs: docs/rbm.md 11.x, docs/initial_investigation.md AIFF parsing (resolve the sub_100054D4 [G]), docs/original_bugs.md (out-of-bounds read, hang).
5. Verify: the four mods and the 317 folder recrypt into scratch; render a song with one; corpus 175/175; the thirdparty gated test; fmt, clippy, cargo test --release.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a6eefbbe69a7e91c3

RE, 2026-10-30. The idalib MCP server was not available in this session (no idalib tools loaded), so the DLL was read with a capstone disassembly of ReBirth.dll as orb-testkit extracts it from ORB_CD_ISO (SHA-256 pinned, cd::DLL_SHA256), in the scratchpad; the user's .i64 was copied to the scratchpad and not opened. Facts:
- sub_10004F94 (find chunk; args id, buf, skip, len, &size): seeks the stream to 0 (vtable +0x14 with 0, 0x10004FD2), reads the 12-byte header (vtable +4, 0x10004FF2), decodes it (0x10004A84 / 0x10004B24 by the swap flag this+5) and checks it with sub_100054D4: strcmp(id, this+6 = "FORM" copied by the ctor 0x10004EB0 from 0x1014F600) and FORM size >= 4 unsigned (0x1000550C jae). The FORM type (offset 8) is decoded but never compared: AIFC or anything else passes. A bad header throws (0x1000504F, _CxxThrowException 0x1002E522, type 0x10030420).
- end = FORM size + 8 (0x1000506A, 32-bit). pos = 12. The first chunk header is read at 12 with no bound check (0x10005071, sub_10005474 reads 8 bytes; id + NUL by 0x10003FD0, size big-endian by 0x100040B0).
- Each search starts from the beginning (the seek to 0), so COMM (0x10003170, id 0x1014F590 "COMM", 18 bytes) and SSND (0x10002FC4 via 0x10004F00, id 0x1014F5A0 "SSND", 8 bytes) are separate walks.
- It stops at the first match (strcmp 0x10005084..0x100050AB, je 0x100051AD): stores the size, then skips `skip` (vtable +0x2C) and reads `len` bytes (vtable +4) at 0x100051C6..0x100051FB.
- On a mismatch (0x100050B1): pos = pos + size + 8, +1 when the pad flag this+4 is set and the size is odd, all 32-bit wrapping; then `lea eax,[ebp+8]; cmp eax,end; jge 0x100051C0` (signed): a header with pos + 8 >= end is not read and the search fails (al = 0). Otherwise seek(pos) (vtable +0x14) and read 8 bytes, compare again.
- So the walk never looks past the wanted chunk: the junk after SSND in Black_Steel / Black_Steel_2 / O.S.C (the 8 bytes past an SSND size 8 short) and mellow (20 stray bytes) is never read. A chunk size is never compared with the stream; a damaged chunk before the wanted one sends the walk to pos + size + 8: past the FORM end means "not found" (COMM: throw at 0x10003209.., SSND: throw at 0x10003018); inside the FORM but past the buffer, or a negative (wrapped) position, reads out of bounds (see below); a size of 0xFFFFFFF8 (or 0xFFFFFFF7 odd, with the pad) leaves pos unchanged and the loop never ends.
- The memory stream of a mod sample (0x1001A563 ctor 0x10006FD0, vtable 0x10030824, buffer of the payload size): read 0x10007180 is a bare memcpy from buf + pos (rep movsd/movsb), seek 0x10007200 stores the position, tell 0x1000DEE0, skip 0x10006E60 = seek(tell + n). No bounds check anywhere: a position or read past the buffer reads past the heap block.
- SSND offset: 0x1000302D..0x10003033 `test eax,eax; jle`: skipped only when > 0 as a signed 32-bit value.

Fix, 2026-10-30: crates/orb-engine/src/samples/aiff.rs. `parse` no longer walks every chunk to the FORM's end; `find(d, id, form_end)` runs sub_10004F94 once per id (COMM, then SSND) from the start: first header at 12 unchecked, stop at the first match, step pos + size + 8 (+1 odd) wrapping u32, unfound when (pos + 8) as i32 >= (FORM size + 8) as i32. A header outside the file (where the DLL's memory stream would read out of bounds) is an error; a walk that repeats a position (size -8; bounded by d.len() steps, since every header read lies in the file) is an error (the DLL loops forever). FORM check as sub_100054D4: id FORM, size >= 4; the FORM type is no longer compared (it was "AIFF" only). The SSND offset is skipped only when > 0 as i32 (it was u32).
Tests: aiff.rs unit tests junk_after_ssnd_is_never_read (SSND 8 short + 8 trailing bytes with a huge size; COMM, INST, SSND + 20 stray bytes; SSND before COMM), a_junk_chunk_before_ssnd (size past the FORM end -> missing SSND / missing COMM; wrapped negative -> outside the file; FORM size past the file -> outside the file; -8 -> never ends; an odd damaged size landing on SSND -> found), the_walk_bounds_and_the_form_header (header ending exactly at the FORM end not found; FORM size 4 still reads the chunk at 12; size 3 rejected; AIFC type accepted; negative SSND offset not skipped). tests/samples.rs: the AIFC assertion now expects "outside the file" (12-byte file, no chunk header).
Old behavior on the same inputs: the four mods failed with "the <junk> chunk runs past the end of the file".
Verification (scripts in the scratchpad, task155/scripts/verify.sh, render2.sh):
- `orb mod recrypt --iso CD.iso Black_Steel.rbm Black_Steel_2.rbm O.S.C.rbm mellow.rbm -o four`: 4 recrypted, 0 failed (before: 4 failed).
- `orb mod recrypt --iso CD.iso "~/rb-stuff-peff/ReBirth Mods" -o all`: 317 recrypted, 0 already recrypted, 0 failed (317 .rbm in the folder, find -iname).
- `orb render` of the 18 shipped songs with --mod each of the four: all exit 0; e.g. Alloy Cowboy Resurrection differs from the built-in kit render with each of the four (the mod's samples are used); G-Spot differs only with mellow.
- Corpus: `orb mod recrypt --iso -o shipped` then `orb compare --manifest tests/refs/manifest.json --mod-dir shipped`: 175 of 175 equal (unchanged).
- Gated thirdparty (ORB_THIRDPARTY_MODS_DIR=~/rb-mods/mods, ORB_THIRDPARTY_SONGS_DIR=~/rb-songs/songs): `cargo test --release -q -p orb-cli --test thirdparty -- --test-threads 1`: 2 passed (unchanged).
- The COMM-missing throw is 0x10003209..0x1000321B (re-read).

Failed first full run: crates/orb-cli/tests/recrypt.rs a_corrupted_sample_is_caught_before_anything_is_written damaged the FORM type (bytes 8..12), which the DLL never compares and orb now ignores too; the test now damages the "FORM" id (byte 0), still in the clear part of the payload, and the failure still names TR808BD.aif and AIFF.
Checks (ORB_CD_ISO set): cargo +nightly fmt --all --check clean; cargo clippy --all-targets -- -D warnings clean; cargo test --release -q --no-fail-fast: 632 passed, 0 failed, 3 ignored.
Docs: docs/rbm.md 11.2 (the chunk walk paragraph, the memory stream, the updated Open item) and 7 (a defect bullet); docs/initial_investigation.md AIFF parsing (the sub_100054D4 [G] resolved to [C]); docs/original_bugs.md OB-22 (new) and Q-3 (junk after SSND, the stream's missing bound).

Review round 1 (78b646d, send back, text fixes only; the reviewer re-read the addresses with capstone 5.0.7 and confirmed the logic):
1. aiff.rs `find` doc: the stop is 0x100050CD..0x100050D6, the jge at 0x100050D6 (was "0x100050CD jge").
2. orb-cli tests/recrypt.rs: the comment of a_corrupted_sample_is_caught_before_anything_is_written now says the "FORM" id is damaged (was "the decrypted COMM chunk").
3. Final summary: the parent is 0e7ccc8 (was 587b17d).
4. original_bugs.md OB-22: a Binary line names ReBirth.dll (it sits in section 5, Mods, which is otherwise Rebirth.exe).
5. original_bugs.md header: OB-22's addresses were read with capstone and confirmed by the reviewer with capstone 5.0.7, not llvm-objdump.
Checks after the fixes: nightly fmt check clean, clippy -D warnings clean, cargo test -q -p orb-engine -p orb-cli all pass.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The AIFF reader now walks chunks as ReBirth.dll does, so junk after COMM and SSND is never read, and the four mods that failed recrypt.

RE (the disassembly of the CD's ReBirth.dll, capstone, because the idalib MCP server was not available this session): sub_10004F94 starts every search at the file's start (seek 0, 0x10004FD2), checks the header with sub_100054D4 (id FORM, size >= 4 at 0x1000550C; the FORM type never compared), reads the first header at 12 unchecked, stops at the first chunk with the id (0x100050AB), and steps pos + size + 8 (+1 odd) in wrapping 32-bit arithmetic until pos + 8 >= FORM size + 8 (signed jge, 0x100050D6). No chunk size is compared with the stream, and the mod sample's memory stream (vtable 0x10030824, read 0x10007180) has no bound check. A damaged chunk before the wanted one makes it missing (a throw) when it steps past the FORM end, reads out of bounds when it lands past the file or below 0, and loops forever at size -8 (OB-22). The SSND offset is skipped only when > 0 signed (0x10003033).

Change: crates/orb-engine/src/samples/aiff.rs `find` (one search per id, the DLL's step and stop; orb errors where the DLL would read outside the file or never end), the FORM type no longer checked, the signed SSND offset; 3 new unit tests with synthetic AIFFs; tests/samples.rs and orb-cli tests/recrypt.rs adjusted to the ignored FORM type. Docs: rbm.md 11.2 and 7, initial_investigation.md (the [G] resolved), original_bugs.md OB-22 and Q-3.

Numbers: the four mods (Black_Steel, Black_Steel_2, O.S.C, mellow) went from 4 failed to 4 recrypted. The 317-mod folder: 317 recrypted, 0 failed. 18 shipped songs render with --mod each of the four (exit 0, the mod samples change the output). Corpus 175/175 equal. Gated thirdparty: 2 passed. cargo test --release: 632 passed, 0 failed, 3 ignored; fmt and clippy clean.

Branch worktree-agent-a6eefbbe69a7e91c3, one commit on main 0e7ccc8 (review round 1 amended it).
<!-- SECTION:FINAL_SUMMARY:END -->
