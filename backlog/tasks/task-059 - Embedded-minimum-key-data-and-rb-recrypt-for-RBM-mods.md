---
id: TASK-059
title: Embedded minimum key data and rb recrypt for RBM mods
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-27 07:44'
updated_date: '2026-10-27 09:56'
labels:
  - rbm
  - cli
milestone: m-2
dependencies:
  - TASK-033
  - TASK-062
references:
  - docs/rbm.md
priority: medium
type: feature
ordinal: 33700
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Decided by the user on 2026-10-27, superseding task-053's "nothing bundled" for this purpose: rb embeds the minimum amount of RB20FUL.DAT key data, and `rb recrypt` rewrites a .rbm so that it uses only that embedded data, so recrypted mods load without the user's CD. Mods that are not recrypted still need RB20FUL.DAT (task-053).

How (docs/rbm.md sections 5 and 6; discussed 2026-10-27): the two key pages are chosen only by sum[j] % (0x8000000 - 2048), where the sums start at {0x2000, 0x3000} and add the big-endian u32 words of the first 1024 bytes of every EMBF chunk (sum[0] the first 512 bytes, sum[1] the second), and those first 1024 bytes are never scrambled. Descrambling is buf[n] += table[(key&1)*2048 + n % P] for n past 1024, with key = the low byte of the chunk's CAT position (HEAD = 0) and P = (17*key % 1925) + 123. So a rewriter can: (1) decrypt the mod once with the real key table (the user's RB20FUL.DAT); (2) add a padding EMBF chunk of at most 1024 bytes (never scrambled), under a name outside the 46 sample names, whose words make both sums land on the chosen fixed offsets (any byte offset in the body); (3) re-scramble every chunk past byte 1024 against the fixed table, keyed by each chunk's new position.

The minimum: steering both sums to the same 2048-byte window needs 2048 bytes. Fewer may be possible by placing every scrambled chunk at a CAT position whose key has a short period P (key 0 gives P = 123, key 114 gives 136, key 227 gives 132; key parity picks the table half), padding with tiny unscrambled chunks; that needs many extra chunks, which the original may reject. Find the smallest amount that the original app still accepts, and choose the window (e.g. one a shipped mod already uses). Embed it in rb (a generated constant with its source offset and the RB20FUL.DAT SHA-256 it was taken from, checked against RB_RB20FUL by a gated test).

Work: `rb recrypt IN.rbm -o OUT.rbm [--rb20ful PATH]`; rb's mod loader recognizes a recrypted mod (its sums select the embedded window) and loads it without RB20FUL.DAT; a recrypted mod must load in the original ReBirth unchanged — verify on the test machine that an app export with the recrypted mod equals the export with the original mod (same song, fitted N; tools/rb_export.py, tools/rbhost/fit.py); and the golden test covers recrypted versions of the corpus mods (the 8 mod songs render identically with the recrypted mods and no RB20FUL.DAT). Document the scheme in docs/rbm.md. Never commit a .rbm or more key bytes than the chosen minimum.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 rb embeds only the chosen minimum of RB20FUL.DAT key data, with its source offset and the DAT's SHA-256, checked by a gated test
- [x] #2 rb recrypt rewrites a .rbm to use only the embedded data, and rb loads recrypted mods without RB20FUL.DAT
- [x] #3 A recrypted mod loads in the original app and its export equals the original mod's (test machine), and the corpus mod songs render identically with recrypted mods
- [x] #4 docs/rbm.md documents the scheme and the minimum chosen, with what the original accepted
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. crates/rbm: model the two readers' keys (DLL: ordinal low byte, for the first EMBF of each of the 46 slot names; exe: the u16 ordinal, for every other file) in Rbm::file; KeyTable gains per-page coverage (bytes known from the start of each page) so a partial table errors instead of garbling.
2. crates/rbm/src/embedded.rs: the window W (a page offset a shipped mod uses, PBE2's page 0) and the L bytes DAT[W..W+L], with the DAT's SHA-256; KeyTable::embedded(&rbm) when both pages are at W.
3. rbm::recrypt(&Rbm, &KeyTable) -> bytes: decrypt every file, then lay the CAT out again: the leading non-EMBF chunks, a 1024-byte pad EMBF (never scrambled) whose two words steer sum[0] and sum[1] to W - 256, then each scrambled EMBF at the next ordinal whose reader's period is at most L (fillers: zero-size '    ' chunks, the IFF filler id; unscrambled EMBFs fill gaps; same-name files keep their order), the trailing non-EMBF chunks; re-scramble against the embedded window; check the EMBF count <= 200.
4. Choose L: build recrypted shipped mods at L = 123 (113662 chunks); test in the original app (a copy under C:\re\rb with the recrypted mod in its Mods: loads, panel graphics, export equal to the original mod's at the fitted N). If the app rejects it, step up the curve (132, 136, ...).
5. rb-cli: rb recrypt IN -o OUT [--rb20ful]; load_mod uses the embedded table when it covers the slots, else RB20FUL.DAT.
6. Tests: unit tests (synthetic), gated tests: the constant against RB_RB20FUL (offset bytes + SHA-256), recrypted shipped and third-party mods give the same files and samples, corpus_mods renders the 8+3 mod songs with recrypted mods and no DAT.
7. docs/rbm.md (new section: the exe key, the 200 limit, the scheme, the minimum and what the app accepted), docs/original_bugs.md (the 200-entry overrun), tools/rbm.py key per reader.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a58e6470e9843cd32

Research (Rebirth.exe.i64, read-only scratch copy):
- COpenMod (sub_41813E) keeps 200 EMBF entries of 276 bytes at +1692 (loop at 0x4182ED..0x41830D, v31 = 200) and writes entry [count] with no bound check (0x418772, count word at +56892): a mod with more than 200 EMBF chunks overruns the table. Non-EMBF chunks take no entry but are counted by v59 (int16, 0x418832).
- The exe descrambles its own files with a different key than the DLL: sub_41678A (the open mod's file reader, used by sub_41703A for the panel JPEGs %d.jpg and Splash.jpg (12999), and by sub_417022 for the demo song, called from sub_419E92) calls sub_4145FE -> sub_4148DF(buf+1024-name_len, size-1024, word Str+272), which computes P = (17 * (key & 0xFFFF)) % 0x785 + 123 (0x4148EE..0x414905) and base = (key & 0xFFFF) % 2 << 11: the key is the whole u16 ordinal, not its low byte. The DLL's samples use the low byte (sub_41693C Str[272]). Same for ordinals < 256 (every existing mod); for ordinal 256 the exe's P is 625 and the DLL's 123. Parity agrees.
- The iterator sub_49F63C: next = start + size + ((start + size) % 2); kind 0 for any id other than FORM/CAT /LIST (sub_49F477). A zero-size chunk is fine.
- sub_414620 (0x4147FE) is the inverse (sub dl): a scrambler reached only from 0x4145F3, outside any function [G: dead code of the packer].
Trade-off (scratch py/curve.py over the 4 shipped + 172 third-party mods: at most 46 slot files and 60 exe-read files scrambled, at most 106 EMBF): with both pages on one window of L bytes, the chunk count needed (fillers between files) is L=123: 113662; 132: 11551; 136: 8154; 140: 6343; 174: 2154; 293: 630; 2047: 107.

Implementation: crates/rbm/src/embedded.rs (WINDOW = 118280160, PBE2.rbm's page 0; WINDOW_LEN = 123; DAT SHA-256 62071e99...2797), crates/rbm/src/recrypt.rs, lib.rs (Reader Dll/Exe, period, key_need, KeyTable coverage, KeyTable::embedded, Rbm::reader/chunk_key/payload/slots_covered/all_covered), rb recrypt in rb-cli, load_mod uses the embedded table when it covers the slots. Filler chunk: id '    ' (IFF 85 filler), size 0. Pad: EMBF 'rb-recrypt.pad', 1024 bytes, words at 508 (sum[0]) and 1020 (sum[1]).
Recrypted shipped mods (rb recrypt, 0.02-0.2 s each): PBE2 108 -> 113663 chunks, 3623024 -> 4532488 bytes; MSM2 -> 113663, 2183516 -> 3092980; Metallicon -> 98263, 3532098 -> 4318362; Orbit2 -> 113663, 5063584 -> 5973048. Recrypting a recrypted mod without the DAT gives the same bytes (cmp). rbm.py extract of original and recrypted (after rbm.py learned the exe key): all 106 files identical for each of the 4.
Gated tests (scratch gated.sh: RB_RB20FUL, RB_RBM_DIR = the shipped four, RB_THIRDPARTY_DIR=~/rb_stuff): embedded_window_is_the_dats ok; real_mods_recrypt ok; thirdparty_mods_recrypt: all 172 third-party mods recrypt, every file and all slot samples equal, embedded table covers all (e.g. 030microdot 108 -> 107888 chunks).
App check (L = 123), copy of the installation in C:\re\rb\t059\app (robocopy /XD Mods) with only the 4 recrypted mods in its Mods; RB20FUL.DAT from the CD (E: RB338V20) as always. rb_export.py --app-dir C:/re/rb/t059/app --remote-dir rb/t059/run --restore-prf --startup-sound off: RSI, Fonda Workout, Metallicon City exported OK (each mod found by the app's own scan, no Download/Standard box). fit.py --all --mod ORIGINAL.rbm --rb20ful DAT --remote-dir t059h (rbhost with the original mod): one N each among 200: RSI 3584, Fonda Workout 3728, Metallicon City 3648. rb compare SONG --reference APP.wav --idle-blocks N --mod RECRYPTED.rbm with no RB20FUL.DAT anywhere rb looks (RB_RB20FUL unset): EQUAL x3. Panels: rb_export.py --play-seconds 15 (rb/t059/run2) with full-screen screenshots during play: the PBE and Metallicon+ panel graphics (JPEGs at ordinals past 255, exe key = u16 ordinal) display clean. So the original accepts 113663 chunks, and the minimum, 123 bytes, is the choice.
Observation [G]: N - N_t was -503, -176, -444 with the recrypted mods against -28, -40, -5 with the originals (task-031): the export starts later after the song open, plausibly the 113663-chunk walks when the song's mod is opened (sub_41813E); not measured further.

Tests (scratch full.sh): python tools OK (tools: skipped=3; rbhost: 33 OK); cargo test (debug, no env) 255 passed 0 failed 5 ignored; cargo test --release with RB_RB20FUL, RB_MOD_DIR, RB_RBM_DIR: 260 passed 0 failed (incl. the new corpus_mods tests mod_songs_match_with_recrypted_mods and modapp_exports_match_with_recrypted_mods; checked that they run: pointing RB_MOD_DIR at the recrypted files makes them fail on the mod size); cargo clippy --all-targets -D warnings clean; rb compare --manifest without the mod env: 164 of 164 equal; with RB_RB20FUL and RB_MOD_DIR: 175 of 175 equal. Test machine cleanup: C:\re\rb\t059 and C:\re\rb\t059h deleted; every app run was rb_export.py's (it stopped its own apps, restored the user's prefs, sha256 6309526e1574, and released the permits); fit.py took 2 permits per run. main had not moved (merge-base 49725e1), so no rebase was needed.

Review round 1 (reviewer passed 7774dc5; supervisor relayed the user's choice and nits):
- Decision (the user, 2026-10-27, via the supervisor): one 2 KB window and compact mods instead of the 123-byte form. Applied: WINDOW_LEN = 2047 (not 2048: the longest period is 2047, 17k % 1925 reaches 1924 for the exe's u16 keys, and the index is base + j with j < P, so byte 2047 of a page is never read). The bytes are crates/rbm/src/rb20ful_window.bin (generated: tail -c +118280161 RB20FUL.DAT | head -c 2047; SHA-256 2c292d920763bc50b058feb019eccc839369528aa92ac4cab3935b8c744956bc), included by embedded.rs; the gated test checks the bytes, that SHA-256 and the DAT's. The layout now takes, ordinal by ordinal, the first file in file order that is unscrambled or fits (was: scrambled first), so with 2047 bytes every file keeps its place: a recrypted mod is the original plus the pad after HEAD, 108 -> 109 chunks, +1032 bytes, for all 176 mods (172 third-party: chunk delta 1 for every one, max growth 1032 bytes). The 123-byte curve stays in docs/rbm.md 12.2 as analysis.
- Re-run: gated rbm tests 14/14 (176 mods recrypt, files and samples unchanged, idempotent); rbm.py extract of the 4 shipped original vs recrypted: 106 files identical each. App (C:\re\rb\t059 again, 4 recrypted mods only; rb_export.py --restore-prf --startup-sound off): RSI, Fonda Workout, Metallicon City exported; fit.py --all with the original mods: one N each among 200, 3600 (N - N_t -33), 3552 (+10), 3568 (-6), back in the originals' range (-28, -40, -5), so the earlier -503/-176/-444 came with the 113663-chunk form and no follow-up task is needed; rb compare with the recrypted mods and no DAT: EQUAL x3; panel screenshots during --play-seconds 15: PBE and Metallicon+ clean. C:\re\rb\t059 and t059h deleted again; prefs restored (6309526e1574), permits released.
- Nit 1: correction to my notes: the shipped .rbm files are directly in scratchpad/worktree-agent-a210df3f4f51953c3/data (no Mods subfolder); RB_MOD_DIR and RB_RBM_DIR were set to that folder in every run (scratch gated.sh, full.sh).
- Nit 2: rb recrypt reports a re-parse failure of its output (and a metadata error) instead of printing 0 chunks (crates/rb-cli/src/main.rs, fn recrypt).
- Nit 3: dropping zero-size '    ' input chunks and an earlier pad is documented in docs/rbm.md 12.2 (Dropped input chunks) and the recrypt.rs module doc.
- Nit 4: the N gap is explained by the chunk count (above); no follow-up task.
- Tests: cargo test 255 pass (debug); cargo test --release 260 pass with the mod env; clippy clean; python tool tests OK; rb compare 164/164, 175/175 with the mod env.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
rb embeds one 2047-byte window of RB20FUL.DAT (crates/rbm/src/rb20ful_window.bin, offset 118280160 = PBE2.rbm's page 0; embedded.rs holds the offset, the window's SHA-256 2c292d92...56bc and the DAT's 62071e99...2797, checked against RB_RB20FUL by embedded_window_is_the_dats). 2047 bytes cover every key (the longest period), per the user's decision of 2026-10-27 for compact mods over the 123-byte minimum (113663-chunk mods; kept as analysis in docs/rbm.md 12.2). rb recrypt IN.rbm -o OUT.rbm [--rb20ful] adds a 1024-byte pad EMBF after HEAD whose two words steer both key sums to the window and re-scrambles every file against it: 108 -> 109 chunks, +1032 bytes, for every known mod.
New facts (docs/rbm.md section 12): the exe descrambles its own files (panel JPEGs, demo song; sub_41678A -> sub_4148DF) with the whole u16 ordinal as key, the DLL with its low byte; COpenMod's 200-entry EMBF table has no bound check (docs/original_bugs.md OB-14).
Results: in the original app with only recrypted mods, RSI, Fonda Workout and Metallicon City exported and matched rbhost with the original mods at one N of 200 (3600, 3552, 3568; N - N_t -33, +10, -6, the originals' range); rb compare with the recrypted mods and no DAT EQUAL x3; panels clean. The 123-byte form was accepted too (N 3584, 3728, 3648, slower open). All 4 shipped and 172 third-party mods recrypt with every file and sample unchanged, idempotently; the 8 mod and 3 modapp songs render identically from recrypted mods without the DAT.
Tests: cargo test 255 pass (debug), cargo test --release 260 pass with the mod env, clippy clean, rb compare 164/164 and 175/175 with the mod env, python tool tests OK.
Branch worktree-agent-a58e6470e9843cd32, one commit (amended in review round 1).
<!-- SECTION:FINAL_SUMMARY:END -->
