---
id: TASK-060
title: 'ReBirth 1.x songs (MIDI-file .rbs): RE the 2.01 import and load them in rb'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-27 08:19'
updated_date: '2026-10-27 12:17'
labels:
  - re
  - engine
  - cli
milestone: m-2
dependencies: []
references:
  - docs/song_load.md
  - docs/rbm.md
priority: medium
type: feature
ordinal: 53000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: task-058 found that 129 of the 193 distinct songs of the user's collection (~/rb_stuff/songs, read in place from RB_THIRDPARTY_DIR, never committed) are ReBirth 1.x songs: a Standard MIDI File (MThd/MTrk) with the .rbs extension, not RBS 4.2 (IFF CAT RB40). rb refuses them (rbs::Song::parse: 'not a valid .rbs song'); tools/rbs.py likewise. The original 2.01 opens them.

What the exe does (task-058, Rebirth.exe IDA, read-only copy):
- File > Open builds the song with sub_425C4D (docs/song_load.md section 2). It first calls sub_43037D (0x425DE4), which opens the file as a MIDI file (sub_4A0B38, the strings MThd/MTrk at 0x4D2AB0/0x4D2AC0) and reads its version with sub_431341: it walks the first track's events until a meta text event FF 02 whose text is 'File version %s' (sscanf at 0x431481; major and minor must be < 100, else XBadFormat).
- If that succeeds, the version is checked by sub_47DA1C(&unk_4DAD60) (else XUnsupportedFormat, 0x42613B), the song gets the built-in mod (dword_4DA784, no Mods lookup) and is loaded by sub_430BA0 (0x425FCE) instead of the RBS reader sub_434D05.
- The collection's 1.x files carry 'File version 3.1.0 (c)1997 Propellerhead Software, all rights reserved' in their first track.

Steps: RE sub_430BA0 and its callees (the MIDI track layout of a 1.x song: tempo, patterns, song events, the device set of 1.x: two 303s and an 808, no 909/effects?) into a new section of docs/song_load.md with addresses; implement the reader (crates/rbs or a new module) and the conversion to the engine model in rb-engine/load; render a sample of 1.x songs with the original (rbhost cannot load a MIDI song directly: either convert through the app by opening the song and saving it as RBS 4.2 on the test machine, or feed rbhost the messages the import sends) and show rb byte-exact.

Context: the env-gated check crates/rb-cli/tests/thirdparty.rs (task-058) classifies these songs as RBS1X and expects rb to refuse them; it must be updated to expect a render.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 docs/song_load.md documents the 1.x import (sub_430BA0 and callees) with addresses
- [x] #2 rb render loads a ReBirth 1.x MIDI-file song as the original does; the thirdparty check renders all 129 1.x songs of the collection without a panic
- [x] #3 At least 5 1.x songs render byte-exact against the original (through rbhost or the app's export)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. RE (Rebirth.exe IDA copy, read-only): sub_425C4D's 1.x branch, sub_43037D/sub_431341 (version), sub_430BA0 (track walk, CC map sub_43067F/sub_430847, track buffers sub_43005E/sub_4300E0/sub_430163), sub_4304C5 (sysex check + 8-to-7 decode sub_4665CD/sub_4663F0), sub_431747 (decoded block -> msg 36 and the msg-41 blocks), the DLL's 909 sub 74. Document as docs/song_load.md section 12 with addresses.
2. Python reference tools/rb1x.py (parse, decode, message stream in the exe's order); rbsmsg.py/run.py/fit.py/rb_export.py accept 1.x songs (read_any) so rbhost and the app runner can render them.
3. Rust: crates/rbs/src/rb1x.rs (MIDI file, version, sysex decode, the decoded block); rb-engine load::load_1x -> engine Song with a load form (Rb1x: no compressor block, 909 gets sub 74 {0,0} = live enabled 0) and the exe's msg stream; Engine::load honours the form; rb-cli load_song detects MThd.
4. Tests: unit tests on a synthetic 1.x file; env-gated thirdparty check renders all 1.x songs and compares Rust messages with tools/rb1x.py for each.
5. Verify: rbhost renders from the 1.x message files vs rb (byte-exact, >= 5 songs); the app's own export of a few 1.x songs (copy of the installation) fitted with fit.py, to confirm the RE of the import end to end.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a4483836f4c7ad0df

RE findings so far (Rebirth.exe):
- sub_425C4D 0x425DE4 sub_43037D: MIDI file + 'File version %s' (sub_431341, sscanf 0x431481; default CVersion(2,0)). 0x425E01 sub_47DA1C(file, &unk_4DAD60): file >= CVersion(3,1) (sub_4257F3 inits 4DAD60 with 3,1), else XUnsupportedFormat. Built-in mod dword_4DA784 (0x425E17), msg 52, optional 58, sub_430BA0 (0x425FCE), msg 32 (0x426610).
- sub_430BA0: MIDI format must be 1 (0x430CBC). Nine track buffers (sub_43005E, 1284 events each; sub_4300E0 sends at 1284 via sub_430163 = msg 37, no validation of tick/ctrl/value). First event: 909 track (0, ctrl 0, value 0) at 0x430CFA. MIDI tracks 1..n-1 are walked; division forced to 8 (0x430D72) so tick = 24 * absolute time. CC (status&0xF0 == 0xB0, any channel): track 5 CC17 -> mixer 9/15/21 = (v==0/1/2), 27 = 0; track 5 CC31 -> mixer 2 = {0:1,1:2,2:3,3:0}[v] (else 0); otherwise device sub_43067F / ctrl sub_430847 (tracks 2,3,4 = 303#1, 303#2, 808: CC7/10/12 mixer level/pan/send of channel track-2, CC14 pattern, CC15 enabled, CC16.. = ctrl 2..; track 5: CC16 dist amount, 18 dist on, 19..23 delay steps/feedback/pan/on/mode, 24..30 PCF freq/res/amount/wave/decay/mode/on). 808 ctrls 10/13/16 get sub_43132C = v/2 + 64 (bytes 55 8b ec ... d1 f8 83 c0 40). An unmapped CC gives device 0xFF and reads var_3C[255] (0x430FEF): none in the collection.
- Sysex (sub_4304C5): F0 00 00 66 (sub_4A14AD), [5]!=0, [6]==1, [7]==0, then 7-in-8 decode to 0x4000 bytes (intended value of the size formula). sub_431747: 'TinY' at +5; msg 36: song mode +0x12, loop on +0x2E, tempo BE u32 +0x2A, loop start bar +10, loop length bars +14 (ticks x768), shuffle 64 (sub_420A81 default), byte 21 = 1 (0x43186A: Vintage), byte 22 = 1. Mixer: master +9, comp id 0, PCF id from +0x40, channels 0..2 from rec = +0x100+32j: enabled rec[0x17], level/pan/send rec[2..4], dist = (+0x37 == j); channel 3 sub_423560 defaults (1,100,64,0,0). Delay: on +0x35, steps +0x2F&0x7F, mode bit 7, feedback +0x33, pan +0x34. Dist: on +0x38, amount +0x36, shape 0. PCF: on +0x3F, +0x39..+0x3E. 303 j: enabled rec[5]==0, pattern rec[1], params p[16..22] (p = +352+4096j), lengths nibbles rec[7..22] (hi = even pattern, +1), steps p[30+68pat+4s] = note, -, flags (slide/accent/up/down bits 0..3), rest; note > 12 -> up, note-12. 808: rec +0x140, p +8544, params p[16..42] with toms 10/13/16 v/2+64, steps from p[48+64pat] 2-bit unpack (sub_43285C), step s instrument i = v[s+16i]. 909: msg 41 4/74 {0,0} only. No compressor block. No 909 block.
- DLL 909 sub 74 (table 0x101542F0, entry 0x1002BF12): {ctrl, v} stores live +0x1891+ctrl then the record hook sub_10012BC0: like slot 11.
Collection: 131 1.x files, 129 distinct (126 v3.1.0, 5 v3.2.0 files); all parse with tools/rb1x.py; no unmapped CC, no non-CC channel events; 808 step values only 0/1; 303 raw notes 0..13; 127 song mode, 2 pattern mode; max tick 383856.

Implementation: crates/rbs/src/rb1x.rs (MIDI reader, version, Block::decode/encode), crates/rb-engine/src/load/rb1x.rs (load_1x/convert_1x, LoadForm::Rb1x; Engine::load sends the seven blocks then 909 ctrl 0 = 0), rb-cli load_song detects MThd, tools/rb1x.py (Python twin) used by rbsmsg.message_file, run.py, fit.py, rb_export.py (rb1x.read_any).
Checks so far:
- thirdparty songs (RB_THIRDPARTY_DIR=.../t063/tp, 206 distinct): RBS1X 129 rendered, 129 message streams equal tools/rb1x.py; RBS42 77 as before.
- rbhost (original DLL fed tools/rb1x.py's messages, idle 0) vs rb render --idle-blocks 0, WAV sha256 compared on the machine (scratch script all129.py, run.py's helpers, one permit): 129 of 129 EQUAL (incl. the 2 pattern-mode songs Spooks and one more; rbhost exports them anyway). First 8 via tools/rbhost/run.py --remote-dir t060 and rb compare: EQUAL.
- rb's compressor live meter bytes 3/4 after Engine::new are the update's (not 127/0): the 1.x load sends no compressor block, so they stay; test adjusted to bytes 0..2.

App check (all permits, a copy of the installation in C:\re\rb\t060app, deleted after; no Rebirth.exe was running):
- tools/rb_export.py --app-dir 'C:\re\rb\t060app' --restore-prf --startup-sound off --remote-dir rb/t060run exported 11 song-mode 1.x songs: Razor Topz, Eternal Darkness, i feel love, Virus 303, MagneticFields, Galaxy Defender, pre2, DISFUNKT (3.2.0), Venom Surf, speed, Gizmo. First run failed on rb1x.Song1x lacking .usri (added usri.show_on_open from block 12640+343); cut this air.rbs timed out waiting for the song-information box: its show-on-open byte is 1 but its strings are empty and the app showed no box (the app's rule not traced, [G] in docs/song_load.md 12.5).
- tools/rbhost/fit.py --remote-dir t060 --app-dir APP: N found for the 9 audible exports (Razor Topz 3584, i feel love 3568, Virus 303 3584, MagneticFields 3536, Galaxy Defender 3552, pre2 3600, Venom Surf 3632, speed 3584, Gizmo 3616); Eternal Darkness and DISFUNKT exports are silent (all N).
- rb compare SONG --reference APP.wav --idle-blocks N --standard-mod target/Standard.rbm: 11 of 11 EQUAL (scratch cmpapp.sh).
So the import's message stream (docs/song_load.md 12) is what the app sends, end to end, for 11 songs; the app's save-as-RBS-4.2 route was not needed and not done.
Tests: cargo test --release 276 pass (incl. corpus_golden 4/4, rb1x_messages, the new unit tests); clippy -D warnings clean; Python tools tests OK (tools and tools/rbhost discover, incl. tools/rb1x_test.py); thirdparty songs check as above.
Process slip: three small edits were made with shell tools instead of Edit/Write (a doc comment in crates/rb-cli/src/main.rs and in crates/rb-engine/src/load.rs via sed, and the first draft of docs/song_load.md section 12 appended with a heredoc); their content is what Edit would have written and was reviewed after.

Review round 1 (send back: record and commit message; code verified by the reviewer). Fixes:
(1) The two doc comments changed with sed were redone with the Edit tool: crates/rb-cli/src/main.rs (load_song: "Reads and converts an `.rbs` song: an RBS 4.x file or a ReBirth 1.x MIDI file.") and crates/rb-engine/src/load.rs (LoadError::Rb1x). docs/song_load.md section 12 was re-read in full; its one over-long line (12.4, the sub_47DB95 sentence) was reflowed with Edit, and everything else in it reads as Edit would have written it; all further changes this round were made with Edit/Write.
(2) The rbhost-vs-rb check is now a committed tool, tools/rbhost/hashcmp.py (the scratch all129.py it replaces is not needed). Reproduce, from the worktree root: cargo build --release; cargo run --release -p rb-cli -- standard-mod --rebirth-dll original/ReBirth.dll -o target/Standard.rbm; then /usr/bin/python3 tools/rbhost/hashcmp.py --remote-dir t060 --json OUT.json FILE... with FILE every file of ~/rb_stuff/songs that starts with MThd (131 files; the tool skips the 2 duplicates by content). Rerun 2026-10-27 14:04..14:07, one permit: "129 of 129 equal". It renders with rbhost --idle-blocks 0 and rb render --idle-blocks 0 --standard-mod target/Standard.rbm and compares the sha256 of the whole WAV (header included), taken on the machine with rcs hash.
(3) The app result is now stated as 9 audible exports EQUAL at their fitted N plus 2 silent exports (which match any N), in docs/song_load.md 12.5, the final summary and the commit message. Uniqueness: tools/rbhost/fit.py --all --max-candidates 200 --remote-dir t060fit (2026-10-27 14:05..14:07) finds a unique N for 8 of the 9 (Razor Topz 3584, i feel love 3568, Virus 303 3584, MagneticFields 3536, Galaxy Defender 3552, pre2 3600, speed 3584, Gizmo 3616). Venom Surf matches at 80 of the 200 candidates (2000..5184, its first match 3632 included): its export does not depend on the start state at that granularity. A first fit --all attempt failed at once on an rcs name lookup error (gnome.lan) and was rerun.
(4) Test-machine cleanup: C:\re\rb\t060app (the copy of the installation), rb/t060run (rb_export.py's folder), rb/t060 (run.py, the scratch script, hashcmp.py) and rb/t060fit (fit.py --all) were deleted with rcs rm -r (each printed "deleted rb/..."); rcs ls rb then listed no t060 folder. rb_export.py logs: every app start restored the protocol prefs ("preferences restored (sha256 dc34d28e7903)") and every run ended with "user preferences restored (sha256 6309526e1574)" then "released the permits (ok)" (13:47:54 for the failed first run, 13:50:27 and 13:52:12). locky list afterwards: redc-win-run 6 of 6 available.
(5) The DLL table for 909 msg 74 was checked in original/ReBirth.dll, not only in IDA: llvm-objdump -d of 0x1002B91C..0x1002B951 shows `movl 0x101542f0(,%edx,4), %edx; jmpl *%edx` at 0x1002B948 (raw sub byte as the index); the .data1 dwords read from the file are 0x1002B951 at 0x101543F4 (entry 65), 0x1002BE90 at 0x10154414 (entry 73, the data block) and 0x1002BF12 at 0x10154418 (entry 74); 0x1002BF12 reads the controller with `movb 0x1(%eax), %dl` (switch table 0x101544AC, entry 0 = 0x1002BF46) and the value with `movb 0x2(%edx), %al`; 0x1002BF49 `movb %al, 0x1891(%ebx)` stores it, then `calll 0x10012bc0`. Written into docs/song_load.md 12.4 item 8.

Per-song result of tools/rbhost/hashcmp.py (song file name, rbhost WAV sha256; rb render sha256 equal in every line; no song content):
- 01_Wax and Rain.rbs 3939cbbdc798a45c88c2eb5bf326e285b567b6a5e1f6cf8a1f922bce3988a8ea
- 3-60 on Red Planet.rbs de4f03613898d83b7fbbc2af21943dad9f3b0538f74765a4cbb5019d98f84488
- 303 To the MAXXX.rbs 1f4a5ce1e124c7e5c3b17c7016fa9f80d47c474d530e7dc4bbcd9a17dc05954f
- 339 1e.rbs 5ded0d2e95699e8017872957dccd480bc9c04d43bffdef5cdffd622d33a9038c
- 430bars.rbs d2ba58fd67fb66d08deac299cd733d2b809850548ca861a7e7d921e1fdf4c550
- 7gates.rbs 796800daa630cd55f93c6412814e1a9b1d136989f69430fde22745dfc13be4c7
- 88daze.rbs 27c7ae4a1a7fa2fa8280f099a9d7e8de827fae9efc788180c79cbc75c7b25dd7
- 98011401.rbs 9d75623039fee423d609679eb96d9f9528fe0827a8513a98afe4eaf335023049
- =SPEED=.rbs 52022845691cc816b0e0d7fe7101d3137538664fdc545418b8cbb2cfaf7643eb
- Acid-rb.rbs 80988da8762c3ccfd3303364e8a5c0c2403d9ec7b802a16b2487e1e93c6a0e70
- Adrenaline.rbs 6329690cbfb98cd1f3f24008e2063e834f1b9d0be02c090ae478ab698362b1bf
- alien contact.rbs 5495c8af7c7da35d8d9feca2b8b0b03ea2ef7d0b7a4f3d191118882abec03558
- Always Never.rbs 73e6252bf299ff0c16c0d9b5773bca495cb73a9d6c0d54d7c54a4a284f850f63
- AnkhMorph.RBS cba985239b8308d19e0705d50bd21a6d985dbcafefb3c8a351860cdb01da6aa1
- Apples.rbs df94fe9ffe5ceb371876dc1fdd65405dab301ed1d0f31e8cfddcd59e58e0cc7e
- AronXnaton.RBS 82d76b5433e2dd3b767073a6777a92bc1a8b36fed7a0dd9e0cec015c7514c4fa
- Austra.RBS 51aa6f000b9f697f4f0b461dd77ce565b4c1d09551fc3a270ef91d8fbbbd50ec
- Bassing.rbs e3d988c1c1f3f34b69a459fe07644e766f5ae34d07742189831619330574c034
- BloodyMarvelous.rbs 880936e387ae664ae7541cf3e8703d1c30a80beaca6e1924f751f88658ddac24
- BlueEagle.RBS 05776663f2d845d1004a4d3ef938fb559e86393111bd4a713c0f134ffe6b6388
- Cats-eye.rbs 28c170f62e8f30c8156aef05765df7e0818ad17b5ee9f67b648dabb29c33a952
- CDT 1.rbs e359261d44de8a8dc42d14236eb8a4a6df2ad2cc4b4afd68de22130df222237b
- cigarette.rbs 7e3039231bc9f0fe3f6bd066185ab5f3feb0b40ee9a66883560f54213121776e
- cut this air.rbs 1a2fc9744886d36c762550b906219e95dc0688abe686adffcf94a16fd6efd357
- d.J ShaGGy journey from.rbs c076513cc9fe34792508a0670b0746bb8ae4add459c3d616b3fa9be3dc796814
- Daydreams.rbs cb14e91567180a55c88a604b193787c1882820a3b5d06138e14b64fec23e4e98
- Demelish.rbs 5801b4273211372c9a82da2efc122e92c29f34849dcc48011e6465a2dcff8b3c
- DISFUNKT.RBS 58dc8da2724023c56cbb5b0168c1fab9e0eec3b77d2df38d9ac997707e81f06e
- Do it right.rbs 9d9a1e83cc8ee912b10f0c6290098bbd1f7c605f4482814f705bd943c714ae07
- Do it.rbs 67ca0b15207342aaccec8e3a6bf740bf185be635bd03efb66ee6a9f3ebd1404f
- Dontstop.rbs 2c440779aa0547f892ef074a7b6ed7d4ffe705fdf053158f8cd3c8c64e122d50
- Dr. Polyphobia.rbs 098745fe19391c86808698259caece89639d829366d8e050f716156c0d176048
- Dreams Of Infinity Remix.rbs de796e0fc7fd1e0cfa2cac621deca986fc3d07a35ed49687c2274cf44db64194
- DreamsOfInfinity.RBS 3510c4bdf62b11bb258644313f107383e7416a7a331d1b566382186bc56b919e
- Dunno.RBS 665df02f2c3564b4aa763174d5db8f8ce95701c29fd75b706e42859fdd19eb0d
- easyway.rbs 6072250cce5016d325a95a36ad18941a372701de0397162239fb7544a069d287
- Electric Sheep.rbs 42b4e38bb29ab98b473a012c73e659860836035ebfb14cf62ce1d2e7b75907a0
- ElectroStatica2.rbs feb9ae3b3007f0f59ca5d0f35eccd78e2feb553704c7fa9ffc968def593b1f30
- Embrio.RBS copy 8166816cff7a1d13c18867e03c1af2091543d876165fa8b0cdf01532990c145c
- Eternal Darkness.rbs 66480fa5d55367b92e5ac17ed070f9dd9219ee1660932c7aa5abcce05b54e6de
- EuroPopPartI.RBS f5df47d7e3cfec3d8de65b659f23fb08eab5faec48b8e0d253b9e67e77a666b0
- Extraterestrial Beats.rbs 6c5de655056e516ed03118a1120c9d363d8431b53fdf472bd91bd6ab1a73c3db
- Fcknfemales.rbs 27488cab0a148a75745070727aaabb222ba778259ddd13d38903e50e7ba681a2
- Fdim+fukd.rbs 31e1c18db10ac71a9e49d4f3279aec7ff6d93a902ae9de857cc438764b3c52fd
- Fearful Symmetry.rbs 224bd3b8fc80937bd578516a6a88fc5e764525d0d5e8fe9cae6909e282977073
- Feeler.rbs db6a83b8e206f11742ce25b74036b11c5ce9cdd654e2783e4df239087770d94b
- Fighting Chick.rbs 08388289e2d03f0e2f59d487977cb04414c93094edc04877857263781fba9c10
- Finereader.rbs 2a7b5779568a57f407f254dba20d43cf84dc34e837c9447fcc412fcef70aa2d5
- food.rbs 3121a593c81faf61f50126db88abd27be3c9a349db3f824c4f0a007c3b7a1fa1
- footsteps.rbs f75a45bebdee4c2a0638c9e9e7396f7f4bf1208c58d0786a9d18b8316475a487
- For an Angel.rbs dcd1e4fb8c570342223f6c9b22b6211079991370670f7dd91002506c9e5b4f83
- For Les.rbs c4920f652619c5cb084486e53280e7dd36e22c7bdef1f9379bf4c85c03849471
- Fractalnature.rbs 3075be0fc574e83e7dba06a85f3da3cbad188765bb36b44112460875c4921dd3
- Futura.rbs 7ab88c4b7121050ed43167fd262956262478e8cdb184a8410489fb4999e1cb44
- Galaxy Defender.rbs bd57bd86ff52c99b636e8b4ea027a283df4dc999bfd6e5b607e111da4bc028eb
- GeeGee.rbs 0c2d3c3a9ab44ed1447fd6da4a4eb8da6340cca24ea921090b87b4d562c5b510
- Get Ready Now.rbs cb95caa8f35f0dcb1194824f5e2f3623fcacd394a1b37b2c21e49f96f1fc6193
- Gizmo.rbs 3eec490813b8d6dd296b25e2e699b833b251f53de20e71d0d69c764fd1752aed
- Hummer.rbs 947f273cd23c24f230e9af2e63f09a3d2c7c24eb3b8a3a3ee2fa93c26efb32a6
- i feel love.rbs 66d2ed893e852563f05218f5cc894d16e6c3a8f0eebdddf0ba1ff4634fcf825c
- I Got An Acid Rythm In My.rbs 8b4d5cd6501bb92111a154cb4795f5ef6196605a64b80eb933c0f387447405a4
- Invention15.RBS 9b3a91400afacbd70ab9e5273018a2d1747076df92c6529c9b0a3426f8cfcd69
- Isopropyl Alcohol.rbs 3cf531c0e6d79ce018f6b030c412b7a0eb10569efb992e18ac004a9e466937af
- Koma.rbs 8fc68e80de68f761dbfa1280dd4c41703ebd5873248652414914bad60015ab14
- LeppardsEye.RBS eb5bba11fce49de939d248f5546c05c95f30ac0face1c280dfa4234ba3727f27
- loongday.rbs d4563309b5e107a0de7edfd02723c6a144ea0759b815ab805a8eae2fd60ec780
- MagicPulse.RBS 5753c124577f534013e339c9a3d0075c391a1ef2daedde6860f7b928f86078a4
- MagneticFields.RBS 721ce52d3f258181807989350907f02c2c8cc25e5fe615a4f760c1905fb13fb8
- Malaria.rbs 5ba421186815b960980a2f556f1391d8ae2555125a79986ef467aa6ec1be5236
- Micx.rbs a2c6da541f4e129c66e8c1e3211fc278d9248050c684073ab5034714357a95f8
- MonoTonic City.rbs 16635481f86e26939192590864802e7d8691e47ac5606ea2a5fb012b4b6a84a4
- Moonchill.rbs 4d4286a3752e56f86b542ae79522077de23fa98a165ae4f9ac3b2cc71fa3e7fa
- Moonflight.RBS 682785d3dba019ba917c53b77768b6b2b66ee8cd427fe0942852cdbadcfed465
- Mrs informed.rbs 310a2869d263d459a6f0018643a59441a59d8a9e20712c324d7e21bf756218d0
- My1st.rbs 71912b9243c51d1a2f5570787af9f696b539f1e109fcefca4f70591f236977aa
- Mysterious Ways.rbs 0ecb02e8cef5b16a87ea5c9ccc66efa7fe315f5f2369bca07aa167af87617d2b
- Narayan.rbs 6950536ab3f19fbd452d33ea72f57a994309646c3cd00f193d5ef1c7061978b0
- No Thrill.rbs e32716e9b29be7677b4dacb3529922988fb37ec9eae8213339831bf522410196
- Odesey.rbs 7e12478f249c8afe778e2ecd02a4096b38326de4b28d520731834219f525a827
- Oilslick.RBS 589cfce6222079e11aef663994db06391b5207beb367718c25317d0b165d3665
- Panchaia.rbs 6294f9beb6c5b591a5591ef0204c159206008f4e89d8b32ed18a781e5008ddaa
- PartyOn.rbs 988ea6a98b83eca0afee6a72958296737a2cf16eaa3879eaee3e6c24903a4deb
- PlanetWobblePart3.rbs 07ed84e2ca75863182c37fe9952196bed5a7a93a244cafc5b0da821decb2c4a8
- PlatonicLove.rbs 0844c4ae22611c4288a418e1383a7cb9f5b98fa1e74e3ac389710a86ea83651c
- pre2.rbs b1e6edce8208dab0188fde55b7c62e7afe0bc4d328995390d83c252f248d65c7
- PsychNerd-Extrapolation.rbs 97a7daea6c1ca2a0f658151a4bf994998c903d6554a6e72a13fa6aa027556d5f
- Pumpin.rbs 6be2ce8008da78b966d52d3db30b0fa8ce8ae024b4a5c0f594a5dbc0cb3c15e0
- PUNKIE.RBS d7fbe0ce0f6b1c8e2f5e94be8256f9d7f4552ecc485216c6b579032e924c1578
- Rage.RBS 8852286b417643f6a6c1c54aa2732b663b0e39913aa91538c1d05d1c9ff054bf
- Raw (Third Mix).rbs 4aadb6d351c1149e1ed9bfb60a39bba45561b66c50bebfcaaa03932191da6c8c
- Raw.rbs a8eb59c634020ae55f0d8117145f0c9cd771289d4f87a426fa1af5102779a66e
- Razor Topz.RBS 7dd60bb5b18838ca3431c480be8e64eeedb26f8af1fb3975b73f62a8a248ce40
- Return.RBS 67811bd71982ca4c40bc770f05b94098a5cd221c720029acda7487a9661400f4
- RidersOnTheStorm.RBS 07d366f1374bbe8e3a4c02725f302f41411c06eb4edbdf310acd972f340e3d47
- Ripples in Space.rbs 086dd48da63781fdb8fdb1575adaa5d8eefdacb7b19141555a7c2fff9686b84d
- s.o.s.rbs 2aaf8d7e37098eee7fad97b80e36e97baaa272e0f074d4eb822c34d52bb80716
- Shake That Thing.rbs 6fa4f7928d75cb7a97fca81af42b0d22b1b924ba76290130d63b1ddae4c00152
- Skank.rbs 85a3c1d46cbdbe9554a1793e4a80b323526b31b110ac7b7a8bb198b276be198b
- Slow moves.rbs 83c74a98df20e8b63aea3601e03c181ec858ca8720dba26a34eaf4727d7216ef
- slow_b.rbs 93dc91d27e5c26d1b28754048963b288df542489795675a20477212dd98f0576
- Smoker.RBS copy 051f910e2c5e1edaf46df5161917c99344c8b48c434827c7e200ebd55dcfb50d
- SnivlemsNoCida.RBS a2c5a44328a43a7ff9a2d613edebdbc8b8540af5bbba775bcfcd00d13112095f
- Soundaxe.RBS acfdbd6902f61c1a10b8ee7a6745331c9b70651aacaf5a7cc1ea9c8bfc9ab7b8
- speed.rbs 982e3e353f4456fb306e56ec2f4a0439bdf6f4482be81d2ee5fb65819e9f9dad
- Spooks.rbs cabdb54a604b9f528106c84605fbff787843f90733c0b4db7abba769f5c4841f
- Squeaky clean.rbs 1980bae950d5506085b15889cb07126d71e46830fec7d69c26c58dc7408c9b2b
- Sun ce SJA.rbs 40f3ffb53c6b9eb4e68f4077fd0169cce07779f6aded6d9f744c5556b279b41e
- Sunbeams.rbs e7f23800de734965b0c25332fd9acc38c179e88915c74122441a832fe181e41a
- Tech'r'Us2.RBS c241f9aa6fff678bca76e5f6826baa628f534535ba4da1d4e76341d440969f39
- The Dark Side.rbs ec16936d9ea1cc1e9bfaf0c693ce61a3642bb8a4d5d78ac58227c2c4fac2284e
- the fun way.rbs b827de2e62ebe4ef36c352745eb740e23334fbb6c640c5365f376bb742ac6548
- TheBirth.RBS 25e1868256717cd3edfe8d187048f1941317805494fbf09a432cddc57e8474fa
- Thick Pan Techno.rbs a9637b78e5dfc76bda56262972ef83282ba714ae1526341fb26d8ebb1ad930f6
- Trancehousidental.rbs 9dd58630bc54144d33a9cca91a7d130d3a2fb06ffa0b106200f4b64d68bfa8ed
- TrAnCeMiSsIoN (vol 1).rbs e31007d6d688d17c999402527b61a1263a7a33d880f613f77046820a76839319
- Trancio Parade Remix.rbs 72c6f0c1783b0ae37e7ab4c29df3bd3b4e21a76be8ffe8072ce89b86226ba624
- Trancio.RBS copy e297213d8a2095da58b8f79aa975acc70b20e1907698fbc1fb9773263e1300f1
- Trancis.rbs a1e1275eb3ba9cd86baef79ffc23b3d17b9273c6a13a7f5538114fbddb18c806
- TrashTech.RBS 838797458e0fc470030ae1f62a3015d9eb18124c7a542d4e1b2337dacc4362d0
- Untitled Song.rbs 464ec222828ec5107d8928b6c98f7530eb0328d8e9617df0f4fd8f8abd94be33
- Venom Surf.RBS befe02920880aba52d92d3a5040e58c468c4cbf10a2d9718e88c2416913bfa55
- Virus 303.rbs abc6b2167d4fd4dae430ff589e6c9ff0ecc3fe4e33825b935e6485ddfd2f36f8
- Vulcano.rbs f3dec1eb5806c32c5d32c8301d3680066ed98bd1f655a2839f6a601cee732f33
- W. Korsakoff syndrome.rbs 3f4f45061ae22513419f82361992bda9a2026ce794ca235601ecd39ed9db45b0
- Watching the Birds.rbs 2784fcc9e26cbf81a671c7c9ebe732c9462756fe5886b1a34756c01281c909c2
- Wibble1.RBS 4b7729b0cc0b2247e291830e5b359f799f2948431a48311a1f6981d008908343
- Wibble4.RBS f7ae43c2b1952fa2e8c3d726bb27a4588581bcd2a59a8b121f317d02d1f585ff
- X-Power.rbs ce588ea284bf666bfd4ea5c74d13de320dd127be6209f6f779b0dd0d9f967cd4
- You Know.rbs c69df912664ecdf34e1be7899366399ae796de7c1a8642dbdc280d143635ca74

Review round 2 (send back: wording and one tool fix; code and tests verified). Fixes, all with Edit/Write:
(1) Venom Surf: its export matches at 80 of 200 N, so it pins no N and is weak evidence. The round-1 note's inference that its export "does not depend on the start state" is withdrawn: a flat match region shows only that the fit is insensitive to N. docs/song_load.md 12.5, the final summary and the commit message now say: 8 audible exports match at a unique N (rb compare EQUAL at it), Venom Surf matches but not uniquely, 2 exports are silent; the "what the app sends, end to end" conclusion is narrowed to those 8 songs, with the 129 of 129 byte-exact rbhost result.
(2) docs/song_load.md 12.4 item 8 now gives the range guard before the table: 0x1002B932 movl $0xffffffbf, %ecx; 0x1002B937 addl %edx, %ecx; 0x1002B93F cmpl $0xf, %ecx; 0x1002B942 ja 0x1002C69C (re-read with llvm-objdump), so only sub bytes 0x41..0x50 (65..80) reach the table at 0x101542F0.
(3) tools/rbhost/hashcmp.py: the work moved into compare(songs, client, permits, idle, render, exe), whose finally deletes the remote x.wav and x.rbhm (errors ignored) and releases the permit, also on a failure. New tools/rbhost/hashcmp_test.py (fake rcs client and permits, the pattern of tools/rb_export_test.py): duplicates skipped, an equal run, a mismatch, and an rbhost failure on the second song (Failure raised, both remote files deleted, permit acquired once and released once). /usr/bin/python3 -W error::ResourceWarning -m unittest discover -s tools/rbhost -p '*_test.py': 37 tests OK; -s tools: OK. Tool run on the machine (14:16..14:17, one permit): hashcmp.py --remote-dir t060 cigarette.rbs "Untitled Song.rbs" -> 2 of 2 equal (same hashes as the list above); rcs ls rb/t060 then held only rbhost.exe (x.wav and x.rbhm deleted by the tool), and rb/t060 was removed with rcs rm -r; no t060 folder is left under rb.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
ReBirth 1.x songs (MIDI files with the .rbs extension) now load in rb as Rebirth.exe 2.01 imports them.
RE: sub_425C4D's 1.x branch (sub_43037D/sub_431341 version, >= CVersion(3,1) at 0x4DAD60), sub_430BA0 (track walk, CC maps sub_43067F/sub_430847, nine unvalidated 1284-event track buffers, 909 off at tick 0, tick = 24 x time), sub_4304C5 (sysex check, 7-in-8 decode to 0x4000), sub_431747 (msg 36 with the Vintage sound model and shuffle 64, mixer/delay/dist/PCF/303/808 blocks, 808 toms v/2+64, no compressor and no 909 block, 909 msg 74 {0,0} = DLL 0x1002BF12, checked with llvm-objdump) are in docs/song_load.md section 12; two defects of the original in docs/original_bugs.md (OB-15 unmapped CC indexes the buffers with -1, OB-16 sysex decoded past its end and a foreign sysex skipped twice).
Code: crates/rbs/src/rb1x.rs (reader), crates/rb-engine/src/load/rb1x.rs (load_1x/convert_1x, LoadForm::Rb1x, Loaded::order), Engine::load per form, rb-cli load_song detects MThd; tools/rb1x.py (Python twin) used by rbsmsg/run/fit/rb_export; tools/rbhost/hashcmp.py (rbhost vs rb by WAV sha256 on the machine).
Results: 129 of 129 distinct 1.x songs of the collection convert and render (thirdparty check, messages equal tools/rb1x.py's); rbhost with the import's messages vs rb render: 129 of 129 byte-exact (hashcmp.py; per-song hashes in the notes); the app's own exports of 11 songs: 8 audible exports match at a unique N (over 200 candidates) and rb compare is EQUAL at it; Venom Surf matches but not uniquely (80 of 200 N, weak evidence); 2 exports are silent (match at any N).
Tests: cargo test --release 276 pass; corpus_golden 4/4; clippy clean; Python tests OK.
Review rounds 1 and 2: the record, the commit message, the 909 table check and its range guard, and hashcmp.py's cleanup (with hashcmp_test.py) fixed (notes).
Branch worktree-agent-a4483836f4c7ad0df, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->
