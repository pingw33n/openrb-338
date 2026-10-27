---
id: TASK-030
title: 'RE: the RBM container and its key table derivation'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-27 04:56'
labels:
  - re
  - rbm
  - docs
milestone: m-2
dependencies: []
references:
  - docs/initial_investigation.md
priority: high
type: spike
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
RBM mod samples are scrambled (docs/initial_investigation.md section 7.4): the DLL descrambles each EMBF chunk past byte 1024 with `buf[n] += table[(key&1)*2048 + n % ((17*key % 1925) + 123)]`, but the 4096-byte key table is derived in obfuscated code in Rebirth.exe (0x4158A9, 0x4149EC): sums of the chunk's first 1024 bytes as big-endian u32 with a stride from anti-tamper globals (dword_4DAE58) and an API value, indexing 2048-byte halves read from a file (app field this[325]) at offset 256 + sum % (0x8000000-2048). Find which file that is, the exact stride in an untampered run, and the INFO and HEAD chunk layouts; document the full RBM format in `docs/rbm.md`. If static analysis stalls, propose a dynamic check (a debugger session on the test machine dumping the table for a known mod) in the notes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 docs/rbm.md documents the RBM container (RBM, INFO, HEAD, EMBF), the sample lookup by name and the full descrambling including the key table's source and stride
- [x] #2 The descrambling is shown correct on one real mod: the samples it yields parse as AIFF and match the original's loaded samples (dumped by rbhost or a debugger)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Static RE in IDA (scratchpad copies, read-only): RBM reader sub_41813E / mod scan sub_4158A9 / mod ctor sub_414D18, key sums sub_4149EC, page read, msg 55 packer sub_41693C + sub_420568, DLL unpack sub_10018870, sample table sub_1001A030, descrambler sub_100163D0.
2. Settle the stride from the anti-tamper page (dword_4DAE58 in the page unk_4DAD78 that sub_4065F2 reads from RB20FUL.DAT), checked against the real file.
3. tools/rbm.py: parse, key sums/pages, key table from a given RB20FUL.DAT, descramble, extract, msg-55 builder; tools/rbm_test.py with synthetic data plus env-gated real-mod tests (RB_RB20FUL, RB_RBM_DIR), skipping with a message.
4. Plausibility on the 4 shipped mods: AIFF parse, JPEG/RBS validity, audio smoothness.
5. AC #2 dynamic: extend rbhost with --mod-msg FILE (send msg 55 instead of 56) and --dump-samples DIR (the DLL's 47-slot CWaveSample table), run on the machine under one permit, compare the DLL's loaded samples with rbm.py's extraction.
6. docs/rbm.md (format, derivation, addresses, [C]/[G]); update initial_investigation.md 7.4 and song_load.md 9; original_bugs.md if a defect turns up.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From task-003 (2026-10-26, docs/song_load.md section 9): the file behind app field this[325] is Rb20ful.dat on the ReBirth CD (volume "ReBirth Win"), found and validated by sub_406A79 / sub_4063DF (magic 0x03E1F11E, string RB338V20, major version 4; strings stored encoded, decode s[j] -= s[j+1] from the end); sub_4065F2 fills 2048-byte pages from it at offsets derived from runtime values. The test machine has the CD mounted (drive E:, seen in Explorer on 2026-10-26); read it with rcs by absolute path (doc-001).

Supervisor 2026-10-26: RB20FUL.DAT on the CD (E:\RB20FUL.DAT) is 134,217,984 bytes = 256-byte header (magic 03 E1 F1 1E, then BC 01 01 00 00, then the Propellerhead copyright string, zero-padded) + 2^27 bytes of key material, which fits the derivation's page offset 256 + sum % (0x8000000 - 2048). RB20UPD.DAT is 128 bytes (magic 07 F1 E1 3B). Details and the CD layout: task-036.

Branch: worktree-agent-a210df3f4f51953c3

Static RE, 2026-10-27 (exe = original/Rebirth.exe db, DLL db; disassembly read for every step):
- Container: IFF 'CAT ' of type 'PRBM' (exe IFF reader sub_49F477: FORM=1, CAT =2, LIST=3; sub_414D18 requires kind 2 and CAT size == file size via sub_49F3B4; the type 'PRBM' is not checked). Chunks in the 4 shipped mods: HEAD (256) first, then 105-106 EMBF, INFO (1280) last. No 'RBM ' chunk exists: the investigation's 'RBM ' is the CAT type PRBM.
- INFO: found by id (sub_49F6F2 'INFO'), size must be 1280 (0x4150BC), parsed by sub_419343: text 0..600, ftp 601..801, web 802..1002, menu name 1003..1067 (must be non-empty, 0x415126), version at 1073: tag 0xBC then major/minor/rev (sub_47DB95), else the app's; the major must equal the app's (sub_47DB0B 0x415178). HEAD: no reader found (string 'HEAD' 0x4BB690 has no xref).
- Mod list scan sub_4158A9 (called from sub_40DDFB with this[325] = [ecx+514h], the RB20FUL.DAT file spec): per mod, sub_414D18 calls sub_4149EC on min(size,1024) bytes of each EMBF payload (buffer prefilled 0x65). sub_4149EC: two u32 accumulators init dword_4BB290 = {0x2000, 0x3000} (sub_414987); for half j=0,1: acc[j] += sum of 128 big-endian u32 (sub_465FCA) at buf + idx*stride, idx running 0..255 across both halves; stride = ((dword_4DAE58 & 0xFFFFF000) >> 14) - (((api & 0xFFFF) + 1) & 0x3FFF) - 14, api = [dword_4DFB24+0x174](hwnd of sub_481A59).
- Then 0x415ED5..0x415F2F: table[j*2048..] = 2048 bytes of RB20FUL.DAT at sub_404C35() (=256) + acc[j] % (0x8000000 - 0x800) (sub_414B65).
- Stride: dword_4DAE58 lies in page unk_4DAD78 (+0xE0), read by sub_4065F2 at 256 + ((((GetWindowLong(hwnd,-8)+1)&0x3FFF)+0x4000) << 11). Checked over all 16384 candidate pages of the real RB20FUL.DAT (scratch pagecheck.py): ((g & 0xFFFFF000)>>14) - k == 18 for every k, so stride = 18 - 14 = 4 [C, given the two API values agree in their low 14 bits]. Same page, dword_4DB124 (+0x3AC): (g>>12) - k == 8 for every k (sub_420568 loop count 3*8 = 24 808 records). The anti-debug branch of sub_41FFBC (IDT check) sets the same expression to 0x12 = 18 directly, and sub_423048 uses it as X/18 = 1 (dist amount index).
- Msg 55 (sub_41693C, not 56; 56 is the built-in mod by id): sub_420568 packs 9542 bytes: u16 BE mod id, at +2 the CFileSpec (u32 BE tag 0xF11E5BEC, full path, NUL; sub_483143), at +4802 the 4096-byte key table, at +8898 24 808 records, +9234 21 909 records, +9528 the startup record; record = 14 bytes: present, key, u32 BE payload offset, u32 BE name length (strlen+1), u32 BE payload size (sub_41FFBC). Name lookup sub_416710 / sub_4713D4: case-insensitive, first match. key = low byte of the chunk's ordinal among all CAT chunks, HEAD = 0 (sub_41813E v59, Str[272]).
- DLL: sub_10018870 unpacks msg 55 into 16-byte records; sub_1001A030 loads present slots with sub_1001A500, which reads payload[name_len..size) from the mod file and, when size > 1024, calls sub_100163D0 on payload bytes [1024, size): buf[n] += table[(key&1)*2048 + (n % ((17*key) % 0x785 + 123))] (u8 add; disassembly 0x100163D0..0x10016449).
- rbm.py on the 4 shipped mods: key pages (DAT offsets) PBE2 118280160 / 44302526, MSM2 8878154 / 9419548, Metallicon 17382559 / 49264678, Orbit2 39986275 / 69184465. Every AIFF parses; every JPEG (59 per mod) ends in FFD9 and splash.jpg decodes as a clean 256x256 image; every embedded Demo.rbs passes tools/rbs.py check (byte-exact round trip); AIFF mean |x[n]-x[n-1]| descrambled 310..1202 vs 21252..21898 scrambled.

Dynamic check (AC #2), 2026-10-27, one permit of redc-win-run per run (run.py's OnePermit), no Rebirth.exe running:
- rbhost extended: --mod-msg FILE sends the exe's message 55 instead of 56 {411} (reply 'C'), --dump-samples DIR dumps the DLL's sample table of the mod (list dword_10118280 -> {u16 id, 46 CWaveSample*}; frames at +0x4C, data at [[+0x50]+0x0C], sub_1000A9C0 / sub_10006FD0 / sub_10007210). run.py: --mod MOD.rbm --rb20ful DAT [--mod-id N] --dump-samples.
- First attempt failed: the DLL replied 'K' (code 18) because the file spec tag was mis-converted (0xF11E5BEC); the exe's constant -249670420 is 0xF11E54EC, equal to the DLL's dword_10150388 at run time (logged by rbhost). Fixed in rbm.py.
- Command: tools/rbhost/run.py --mod <scratch>/data/MOD.rbm --rb20ful <scratch>/data/RB20FUL.DAT --dump-samples --tag t030 -o <scratch>/host_MOD tests/songs/diag/songs/303_accent0.rbs; then scratch cmp.py MOD DUMPDIR (DLL slots vs rbm.aiff_samples(rbm.embedded_file(...))).
- Result: PBE2, MSM2, Metallicon, Orbit2: 46/46 slots equal each, sample for sample (184 samples). Mod file SHA-256s equal docs/test_machine.md's installed ones.
- What this proves: the DLL's descrambler and AIFF loader, run on rbm.py's records and table, yield exactly rbm.py's samples; the table derivation is confirmed by content (every JPEG decodes / ends in FFD9, every Demo.rbs round-trips byte-exact, audio smooth): a wrong page would garble every byte past 1024. Not done: dumping the table that Rebirth.exe itself builds (no memory access to the app via rcs); an app export with a mod (task-031/033) would close it.
- For task-037: each mod needs exactly two 2048-byte pages of RB20FUL.DAT (4096 bytes), at offsets fixed by the mod's own bytes; the 4 shipped mods need 8 pages in all (offsets in docs/rbm.md 5.3).
- No defect of the original found (docs/rbm.md section 7); docs/original_bugs.md unchanged.
- Tests: tools/rbm_test.py 9 tests (7 synthetic incl. a sparse 128 MiB DAT stand-in; 2 real-file tests gated by RB_RB20FUL / RB_RBM_DIR, skip with a message); with the env set all 9 pass. Full discover: tools 45 OK (2 skipped), tools/rbhost 15 OK.

Review round 1 (2026-10-27), send back; fixed:
1. Stride check committed: tools/rbm.py check-dat DAT (check_stride_pages: k = 0..16383, dwords at 256 + ((k+0x4000)<<11) + 0xE0 and + 0x3AC) and the gated test RealDat.test_stride_is_4_on_every_page (RB_RB20FUL), plus a synthetic test on a sparse DAT. Run: /usr/bin/python3 tools/rbm.py check-dat <scratch>/data/RB20FUL.DAT -> ((dword_4DAE58 & 0xFFFFF000) >> 14) - k: [(18, 16384)]; ((dword_4DB124 & 0xFFFFF000) >> 12) - k: [(8, 16384)]; 'stride 4 on every page', exit 0. Cited in rbm.md 5.2, song_load.md 9, initial_investigation.md 7.4.
2. rbm.py parse now warns (UserWarning) on a CAT type other than PRBM instead of rejecting, as the exe does; rbm.md section 2 says so; test_bad updated (and a wrong CAT size still raises).
3. rbhost.c: the tag compare casts each byte to uint32_t before shifting.
4. rbhost.c: the m[1]/m[2] after the if/else were load-bearing for the mod path (msg 52 needs the new mod id); removed them and set m[1..2] from the message inside the --mod-msg branch instead, so each branch sets them once.
5. Commit area now tools:.
6. 0x65 padding: Metallicon.rbm has 8 EMBF payloads under 1024 bytes (JPEGs, 810..931 bytes), so real data exercises it and Metallicon still decodes and matches 46/46; PBE2/MSM2/Orbit2 have none (smallest 2335/1203/2083). In rbm.md 5.1.
- rbhost rebuilt: build/rbhost.exe sha256 cb50fe50f54f41410bc74f1018e48efe1489624f937e842a7746c7447332537e (not in the README, which records no build hashes); rerun on Metallicon under one permit: 46/46 slots equal.
- Tests: tools discover OK (3 env-gated skips), tools/rbhost 15 OK; rbm_test with RB_RB20FUL and RB_RBM_DIR: 11 OK.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
RBM format and key derivation solved and documented in docs/rbm.md (branch worktree-agent-a210df3f4f51953c3, one commit; review round 1 fixes amended).
- Container: IFF CAT of type PRBM (type unchecked by the exe) with HEAD (256, unread), EMBF (name NUL file) and INFO (1280: text, ftp, web, menu name at 1003, version at 1073). Samples: 46 names (exe 0x4BB388 / 0x4BB520 / 0x4BB378), case-insensitive first match, missing -> built-in.
- Scrambling: payload bytes >= 1024 get table[(key&1)*2048 + (n-1024) % (17*key%1925+123)] added; key = low byte of the chunk's CAT ordinal (HEAD = 0).
- Key table: two 2048-byte pages of RB20FUL.DAT at 256 + sum[j] % (0x8000000-2048), sum = {0x2000,0x3000} + per EMBF the BE u32 words 0..127 / 128..255 of its first 1024 bytes (0x65-padded; exercised by Metallicon). Stride 4: tools/rbm.py check-dat shows X = 18 on all 16384 anti-tamper pages of the real RB20FUL.DAT. Each mod needs 4096 bytes of key material (input for task-037).
- Message 55 layout (exe sub_420568 / DLL sub_10018870) documented; tools/rbm.py (parse, pages, key table, descramble, extract, msg, check-dat) + tools/rbm_test.py; rbhost --mod-msg / --dump-samples, run.py --mod/--rb20ful/--dump-samples.
- AC #2: on PBE2, MSM2, Metallicon, Orbit2 the original DLL's loaded samples equal rbm.py's for all 46 slots each; all embedded JPEGs and Demo.rbs files decode.
- Docs: rbm.md new; initial_investigation.md 7.4 / 10 item 3 / 11; song_load.md 9; rbhost README. No original defect found.
- Tests: tools 47 OK (3 env-gated skips), tools/rbhost 15 OK; rbm_test with RB_RB20FUL/RB_RBM_DIR 11 OK.
<!-- SECTION:FINAL_SUMMARY:END -->
