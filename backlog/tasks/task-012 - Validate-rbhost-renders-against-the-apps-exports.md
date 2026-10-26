---
id: TASK-012
title: Validate rbhost renders against the app's exports
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 15:55'
labels:
  - tests
  - testmachine
milestone: m-0
dependencies:
  - TASK-010
  - TASK-011
priority: high
type: task
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
rbhost (task tools/rbhost) is only useful as a reference if its output is what the original app exports. Render the diagnostic songs with rbhost and with tools/rb_export.py under the most deterministic app protocol found by the determinism experiment, compare them with tools/rbcmp.py, and explain any difference (it should be explained by the state the app protocol leaves uncontrolled, or by a mistake in rbhost's song loading to be fixed in rbhost). Results in the notes and a section in docs/export_state.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every diagnostic song was rendered both ways and compared; the comparison table is in docs/export_state.md
- [x] #2 Each difference is explained or fixed in rbhost (with the fix's commit referenced)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. rbhost: render all 145 diagnostic songs at N = 9088 (the value of the task-011 session), one permit per run (run.py).
2. App: rb_export.py --restore-prf --startup-sound off --wait-after-open 10, one process per song, in batches (all permits per batch); measure the time per song first and capture all 145 if the budget allows, else at least every README category.
3. Compare every pair with rbcmp.py (exact). For a mismatch, fit N with run.py --idle-sweep around 9088 (steps of 16, outward) --match APP.wav; record fitted N or first difference.
4. Explain each remaining difference (N, 303 idle output, drum counters, delay write pointer) or fix rbhost (rbsmsg/rbhost) and reference the fix.
5. Table in docs/export_state.md (new section 13), hashes in the notes; WAVs stay in the scratchpad.
6. One commit, task To Do -> Done.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a7d4f8be649280156

Work log (2026-10-26). Scratch dir: the session scratchpad, subdirectory worktree-agent-a7d4f8be649280156 (app/, host/, fit/*.json, cmp9088.tsv, timing.txt, table.md; WAVs not kept in git).

1. rbhost, all 145 at N = 9088: tools/rbhost/run.py --idle-blocks 9088 -o host tests/songs/diag/songs/*.rbs (rc 0; rbhost.exe sha256 6f28d8318c876fd5d0834e3940b3fd92a559953cd6f3d916d8a6c8c54e61580a, tools/rbhost at 7838f84, unchanged). About 0.7 s per song including the rcs round trips.
2. App, all 145 (none skipped; about 25 s per song, 62 min): tools/rb_export.py --restore-prf --startup-sound off --wait-after-open 10 -o app SONGS, fresh process per song, 7 invocations: a first run of 3 songs (silence, 303_accent0, 808_snare), then app_all.sh's batches of 24, 24, 24, 24, 24 and 22 songs, all rc 0, prefs sha256 dc34d28e7903.
3. rbcmp.py --json app/X.wav host/X.wav for every song (cmp_all.py): 47 equal at N = 9088, 98 differ. 43 differ from frame 64..194 (songs with 303 notes; max |d| 433..37659), 55 differ by +-1 LSB from frame 2889..44325 (songs without 303 notes; 166..52350 samples).
4. Fit (fit.py): run.py --idle-sweep 8704:9488:16 (49 values of N, no --match so all render), each render's samples compared with the app's; about 36 s per song, 95 min in all, one permit. Result: every song has a matching N, and the whole WAV file's sha256 equals the app's at it. 142 songs match exactly one N; silence, pcf_lp_wave127 and pcf_bp_wave127 match all 49 (all three are all zeros, sha256 0a90d04342bf..., in the app too).
   Fitted N histogram: 9040 x3, 9056 x5, 9072 x25, 9088 x44, 9104 x37, 9120 x16, 9136 x9, 9152 x2, 9264 x1 (pcf_on_909).
   Distinct renders per song in the window: 49 for 140 songs, 43 for delay_straight_fb0, 42 for delay_triplet_fb0, 1 for the three silent ones.
5. N against the runner's timeline (timing.py, sidecar steps "song opened" and "export started"): N - round((t_export - t_open)*44100/64) = 47.6 +- 14.6 blocks (min 3, max 82) over the 142 unique fits; open-to-export 13.08..13.21 s, 13.38 s for pcf_on_909; its late export start explains its high N (9264), and its N - N_t of 47 lies inside the spread. This is an empirical fit, not a mechanism [G]. A sweep seeded at N_t + 48 +- 48 (7 renders) would have found all fits: input for task-038.
Explanations: every difference at N = 9088 is N only (the app's N differs per process; 303 phase for songs with notes, 303 idle output for the others, docs/export_state.md 11.2). The drum counters start at 0 in both (one process per song), no --drum-counters needed. No rbhost fix was needed. Failed hypothesis: "songs without 303 notes match at any N" is refuted: the template's 303 channels at level 100 put the idle 303 output into every export, so all non-silent songs are N-specific; the delay write pointer's own N dependence cannot be separated from it in this set.
Side observations: pcf_lp_wave127 and pcf_bp_wave127 export all zeros in the app (PCF wave 127 silences the 808; not traced, follow-up task created); shuffle_flag_off = shuffle_303_g0 (as designed); at N = 9088 rbhost gave 808_open_hihat and 808_openhat_knobs_hi the same file (063987f1d048) but the app exports differ only by their N (9104 vs 9072), so the decay 127 knob probably equals the template's value (song-design detail, not checked).

Per-song hashes (app sha256, fitted N, rbhost N = 9088 sha256 prefix or "same"):
303_accent0 app 1efcc1325788342319ebb4f425aa604462c89a78313f8f8bfe2272ce5e4bbaaf N 9088 host9088 same
303_accent127 app f23880cf6d629f459134e71b61f5efcbe51ee21f9619ada0960d9fa92cbe7c26 N 9088 host9088 same
303_env0 app 96757f1156f53f3e0b7ad8da212317906d52e43e94a80cdf9975111475da1da7 N 9088 host9088 same
303_env127_decay0 app f0094483c8e5e8b9310219ddf1ee7f7e66440435748d47cd0a1217a5b207ce99 N 9072 host9088 1913a5812f5a5e8b
303_env127_decay127 app e04b490f725ed530326cfb7606add4def6a5bf72b5e091643ed8d7369f3cad20 N 9104 host9088 9196ea6bb16e5b1e
303_res0 app 233c94b17f1bedfb90edec615ad816c056be7e60cf7ddd62b1f9234716b4e3a9 N 9088 host9088 same
303_res127 app b27a3a501f5152a589c0d3785a471ccd1fab438294fd43c72e755eea9083127b N 9072 host9088 8853065e37f34c31
303_res64 app 3f39d74e468e58dc41019b142773587e9ff58ef070a25363a95d8dd0fb77d61a N 9120 host9088 128915730b97e187
303_saw_highcut app b25f951c55be063ad9f94241f679e5327c81920698c3871830024f54046694ab N 9104 host9088 964c0f722521d3a1
303_saw_lowcut app dc2c09cf1a140e27d27dc0129f8db5735120a7d1ca14259aa06a9c6000acdf8b N 9072 host9088 58a4f47ecaab75f1
303_slide app db11fa67f5ef7dc688af8347249198a2ab5b1e5a832cbcb1ec85cfa89e6a0b63 N 9104 host9088 558cc7b9d788e07e
303_slide_accent app 906c6effcb7b95bdc549f63cb49d793f1b1ecc5f8e3ff35725c66b01ba344501 N 9104 host9088 78a3dda2e23e45b0
303_square_highcut app c968a8a8ba29dde7b00a203f6f4f7149f8c4b07c0f292489b6652d3432798bf4 N 9088 host9088 same
303_square_lowcut app 28311f6420da81706ef84f17ae04e2f1de5c73597a9cfbf20b7a29f3f82a3663 N 9072 host9088 84b06d898405a81d
303_tune_hi app a86b2f3b2362d48eb1e5baef9f5816f131c03b449816d18e03e85f9d506eadb0 N 9120 host9088 d04907c959f9f85e
303_tune_lo app 9b765fa0a2b8c9d5ac11259e22d26de48bb3d8b2291e03eeed9a0f7f0689d467 N 9104 host9088 ad41bf3f5f5056f4
303_two_voices app d27b3812eaa390ea0a93d56006266ec758c336ef870286daf2b8989825632868 N 9088 host9088 same
303_vintage_accent app 313a45507fee04b50e0cb11eb24abebc2fb2d33fa0a43c1f392f89fda48cda1f N 9104 host9088 9db2bdd371353c18
303_vintage_highcut app 7a54c5c131a36cb129d922ac4eb167e043698f697b052d88d0466c57f63f8d81 N 9104 host9088 dd86a88eddfec008
303_vintage_lowcut app ee706433f8f8a2132679a7770b89b4dc5936ffb8cefa54ff6854006bfecb2d1b N 9088 host9088 same
303_vintage_res127 app 05379a8965041292c1d01355cc40f80a4c0f150628d67933990eba079ad0642a N 9088 host9088 same
303_vintage_saw app f97f44baea9e3dc73673da4760c963b29808b50572bdf9c17d0a395eae1b1cea N 9072 host9088 77e0b263255aa029
303_vintage_slide app 49e663700b791a04ee7473628c67f7a6bf4432dc574297f4fb56851e65d9cc19 N 9088 host9088 same
303_vintage_square app f3256864bc9cbd1f169bf164bd385fb7a224bfcef858f49d0c0df1e38e615e0e N 9104 host9088 a5585e4acf0a08d4
808_accent_level0 app ceacf47658c13c36a2237a05659b4404a977521d55f37e467d8fadad771bb319 N 9088 host9088 same
808_accent_level127 app ef860e2097f12e8d48aeb847f48008b6b8ce10916edb040ce3262433042e6f73 N 9072 host9088 414a7ff440fda248
808_bass app 1777d26884f26c505b22cad1aef68e717ab1b41878c3188775bb2c94863f3f6b N 9088 host9088 same
808_bass_knobs_hi app 4e481bc1759c659189c6177ae2dea6fcc9cdd479d2d0344ed3ed0143039a9f6d N 9056 host9088 9a3a68849b8ef98b
808_bass_knobs_lo app 7c04b5742c870b0283116f0754846c023440993edcae815ac1e975167a95d77d N 9072 host9088 a8baca4e82f70ae9
808_clap app 7baf8a0ae5ee323432e346b304f7d03dd4db4f95129f0a8ba5e874808cad2ed8 N 9088 host9088 same
808_clap_knobs_hi app 49cee9613c897493096c09211de41efa626a0cd1548cdcfaf8f006079399991f N 9056 host9088 e396ba21711e2480
808_clap_knobs_lo app 4b319516955812feede72d47f462adedde5b73828affbe3d4544d3036251d3db N 9072 host9088 381ef0d2b93d3108
808_closed_hihat app d08d90061aa0522ead415551b7aef80e199ccc97dde09827cd5b4f243ba507a0 N 9072 host9088 7dc776ca89849920
808_cow_bell app d5aaba11b2135297fcfa3dbd485625933dbdb284851e3900893c0d1ccbbcf7b7 N 9120 host9088 0e2e98fc5211ec6c
808_cymbal app 61c364ef6f3473f602185683bc02414bcd468a8f606023661b72a57cf8059f63 N 9088 host9088 same
808_cymbal_knobs_hi app 4db360394e4dd18a57ae8b3b6c9655da3674cacf2163bb7b3cefc8d30ecf636f N 9072 host9088 35b2f6dfb9578737
808_cymbal_knobs_lo app 13ac9a3314f20452a85e50c3881ac06c5e70dbf385ff998ce885a11d82c2c2db N 9136 host9088 448921ae0ce4a514
808_hi_tom app 471b99229d9386e9d46d2f3f9e134b112f8b2ff9543df1b2b54c8488b5367746 N 9088 host9088 same
808_low_tom app 56a4d54cd441ee7bcdaa47843a6b3a63c4f059c3187c991907e116289dfe8e84 N 9152 host9088 fb51a9667b25fc34
808_lowtom_knobs_hi app 9584ea4839e569fe62e78b93e5891c8784a20a962bcc01cd0d236a28b4224e0a N 9088 host9088 same
808_lowtom_knobs_lo app 1133d93a841b6b75af2e7d314803905345f34a085597fd9a2f25feda0082fd34 N 9088 host9088 same
808_mid_tom app 0797ecee08032a21a2de040d74f8b635e965233f68e5aa80ab02e92426225f21 N 9104 host9088 141000c304fbe659
808_open_hihat app 999c011a8c62d326f57c35def2ebc6e1a578de9c4ffdce684bd0c85e54835eb8 N 9104 host9088 063987f1d048106f
808_openhat_knobs_hi app bd463062609750c80ef6b0e3d5c672d5cd1b321f6bb8b89920580613b6c7af30 N 9072 host9088 063987f1d048106f
808_openhat_knobs_lo app 0d5a289800c8d14817f9598e2a9a89287d6827d6000222d18bfedcb52c0837a0 N 9088 host9088 same
808_rim_shot app a258cbe5946d3d962ac9fbf19926146f04a2ceacf283806006ea1036fff37722 N 9104 host9088 1e0af6d230978b98
808_snare app 1a2a932b9518acad6bc29319cefe448318977222ec2092b9348675004fb33bd0 N 9040 host9088 f31175af005b32f2
808_snare_knobs_hi app f9853eaea353eb899a34db00ae0e65bc538dd98d3e332184d82dea97b0b807be N 9120 host9088 dbcb5e4b65e57332
808_snare_knobs_lo app 9ab19d2b0d04158451563e00257629959eea7f397f83ff46c4868917aafd0558 N 9136 host9088 7ac6bfc194d22eaa
909_accent_level0 app 1bcb48792767d0ed223446277794e1a8ff673e3ccbcd33f68806f4d382f59109 N 9120 host9088 80b4b383f21e8119
909_accent_level127 app f82632bd225cc46789e02dcf4e353207e19dfda497c71830c5ae5f1efc264307 N 9088 host9088 same
909_bass app 62b37761b09a6cc439a2dd8432a78a0c3444dc33e196138c0b8792086878c80c N 9088 host9088 same
909_bass_knobs_hi app c923e6688c486881ce472c1394ea44c6e8e0a7a9f7338a0602561bd91ba202b3 N 9120 host9088 8ac598c7118c820f
909_bass_knobs_lo app 578cacfe5148ac89ae8ed28f73f00b72063dc6f0bbcb6ed252805b1b72c56b03 N 9104 host9088 0e13b1c8445b438e
909_clap app dee1bbdad692273592690896396b406a3611d3c883b4bebd433b5f59fb0a1fbb N 9152 host9088 c09219af7cecc0c9
909_closed_hihat app ddce0b504c8ca1a40a1589a60390193bafa7e1dc2de4b162bf7ff017f19d357e N 9072 host9088 343a699b2c22c856
909_closedhat_knobs_hi app 55ddb3004c064cc1f851cb5ceb7e291273e4fbbc5468c778810d544c3b178cfe N 9072 host9088 93a2057b02cf111b
909_closedhat_knobs_lo app f0f65d893468b9d561b398f3ff32c72f49bb2148ee8e352c5cf6a00fb86ac4de N 9104 host9088 21bd25197cd37723
909_crash app ddca672ae7c52ce131d9ef6074415c70d2393a4246ad971d643b718cd6f72831 N 9088 host9088 same
909_crash_ride_knobs_hi app 7aee827e727c8d145572b2d7104a7c31fe159296f7f437ac97ab63efc7bb222e N 9088 host9088 same
909_crash_ride_knobs_lo app e6234ec61aaf91ca1d56116ab00c295715a452154b03f644867830bba2f852bc N 9104 host9088 63cb7b58eddcaa1b
909_flam_interval0 app befaf990677fea9eb112a94969854a849d5490441ea6c08e41cdf5caeaaaf6e2 N 9104 host9088 5a36e72ddbf737f3
909_flam_interval127 app 9017530fce09d3e8288dd62f71eb2fa6da4a445d7850c479a178d641f1ee4350 N 9104 host9088 c7b445140b40387e
909_flam_interval64 app f3a33e24a36373669374fad8cd40341eb0b20d5e8f688dab739313b359131dbe N 9104 host9088 b10e9fc38adf9555
909_hi_tom app dffe5e2043557ffa82bf7016c11a200ccc0eda7175ee8ddbf9ae30d097d0dbe5 N 9136 host9088 360841599b248522
909_hightom_knobs_hi app 04d5d1ff1df9dc371536db3fed4d1ddca23739e905daa7a0da1091eb41804c3d N 9088 host9088 same
909_hightom_knobs_lo app 93a0096703706d7be0128ea2fd6373d881540a1833e781d313b6f969cca59015 N 9120 host9088 77a269c0c51a223f
909_low_tom app 096e4ca24f5eedc8c551d7b0baf4b32ff1a135212f99dd8399e05fe0aaa73390 N 9088 host9088 same
909_mid_tom app 8f42848e7328da31231e5e7e43d005bdbb78bbed2fadde881c1af9676a07a9ae N 9120 host9088 4a2382cd900e1b67
909_open_hihat app d3ce643ced7573fb83fb2668a75c9015be9f3457e6cd08eecbe04f82e373b5a6 N 9104 host9088 663e5613e9ca4560
909_openhat_knobs_hi app dd0421cf4c52f4428b3a1a596250dcd7cbf3964354934a9c96ad6c6dc8650c85 N 9136 host9088 663e5613e9ca4560
909_openhat_knobs_lo app df43b22db910dd294411b9bac933f137f9fcd1e1ec9370e9fbd831a64c9af3cd N 9120 host9088 135ff1f40a08ab61
909_ride app 30f1e6e3e18d7c36132be3d6e49e0d6dd4b32eedae31c74400a46b7eff52cd39 N 9104 host9088 c061b9be94054867
909_rim_shot app 3c97b95c2707030770ff928ba7a0f97506416bc1dd40b650b34b111bc0e8cb5c N 9104 host9088 9ade51bbfdcbc266
909_snare app bb19aa626958b230f0513a8e18572bcf0eeff37a03b0039877538124cb93cbb4 N 9072 host9088 69e4dd1962dfcfa9
909_snare_knobs_hi app cb69e39398c5714eff2c932edc1b2191273fb63eace0b652bac33ed55af883ef N 9088 host9088 same
909_snare_knobs_lo app 9f9f4fdfa0f5211148c9212dccdefc4a6ea6e45279a13584efed1b5de4713764 N 9088 host9088 same
comp_ch_808 app a4c74554723b54c035c7c3092ae5f703ec0b52269a020e57d2ed033ca3368ef0 N 9088 host9088 same
comp_ch_a100_t10 app 0056ba3c333c6af910b4ac10b70315fe115bd4b509140b1efed9cd10007254a6 N 9088 host9088 same
comp_ch_a127_t0 app f75d0c82532ae91f3835be4a0788c6a4cfd7ec33d4749e659ef098d069c34ec1 N 9072 host9088 19e2fd7c4bf9721b
comp_ch_a30_t64 app d2ddf1715ba838927d4c0fcdc83f8b1d5306c4f1303433ed08a15240ed639b4e N 9072 host9088 f927a86119028c7c
comp_master_a100_t10 app 4b9526520619b1f378beb1ba84b1fa5a0c1c9b314c47c99b92ce1eb4fc29e5e6 N 9088 host9088 same
comp_master_a127_t0 app dd3a0e32f87c441a55c554191bff3ebd4a5e65d31761b1fa6ba56b0b982c8587 N 9072 host9088 144dd669b528cbd3
delay_303 app 1b87af67a9394889c11d77a9fa4f4413a6c6dbc6d52780564df272a2ff695db0 N 9088 host9088 same
delay_automated app cbb901e13ba04c5b8d135170894366f468c0f4531d38600e441bf289d93d6280 N 9088 host9088 same
delay_pan0 app ffec7b63fdc224acedd775c0c3b31048414e43c93e30a4c3feb3df24620cbb5d N 9088 host9088 same
delay_pan127 app 2344daf196f986f5fb75c55e31556d27f088b6699da7e449550c0e9096b463a4 N 9088 host9088 same
delay_straight_fb0 app 897c3a2c23c8597bc20e37ab767d96bc64b83040eac94e0b27ea92961257238d N 9088 host9088 same
delay_straight_fb127 app 90b8f931400aec1a19dd90f235550fea683050bacd18eae41f0f8a6933a78e03 N 9136 host9088 766a13c8312410cd
delay_straight_fb64 app dece122a73fc7667ef7427666e24de4bb16ff07f41e2088c00ec39a762d93f4f N 9088 host9088 same
delay_straight_steps1 app 8c7ccd66dd114bf76e298822373c18e4ce999ed507aafa9f7e66d50b6f2a02e6 N 9120 host9088 a8a5499222d6f1d1
delay_straight_steps8 app 4ec0af3fa961858eb0e78b7196f2a7f316214fbef6d9e77ac732a15113ab3ca5 N 9072 host9088 f686a41a61f26312
delay_triplet_fb0 app 3a1bbc4279aa7438b4f5fe562c44fd6df5463f6384d623719d0eda3c1aea9a50 N 9088 host9088 same
delay_triplet_fb127 app 4f0758c75fc3c19a6d0e7f8088f764fe901418400509748e22b9bf24a1f6a8ef N 9104 host9088 8597f020b0b0b5ad
delay_triplet_fb64 app ea7cb17ea4fac9a42bf09df52564e1119a0b6229681cd60d049a448bdbb06f38 N 9104 host9088 c84ab7ee6dfcb51d
dist_a0_s0 app 7ff5cd6c43e548d1ffdcc78017e7d5a9f29eecaef874903046fd56c52f32f224 N 9104 host9088 73c520abca0853dc
dist_a100_s0 app 1025b84dadce841c9e954bb183340a14956adfb3f1d05bbf96a14afd20e2b83c N 9120 host9088 f39b93c86963142d
dist_a100_s127 app 435a92ee730ab22bf88f7f3e877229fd0ab9cfbca84b9805c1f8b6a6ceab6cfb N 9136 host9088 23fd04ef87a4239f
dist_a100_s64 app e09067db77d7ea6d32145e87a2df18eb5fca095e2853140a41344f7be5e538ac N 9072 host9088 ea40794e468b64c3
dist_a127_s0 app 3f22671dd4eabc11db08357071e977ea18ad98ce53dbcf4a93cebdc82ca7936a N 9040 host9088 c6b3915f20e9e496
dist_a20_s0 app 23f36775827f97e1b68d5c13ed9f4caf993e5f6466002421116bd5b0fd42d35a N 9104 host9088 bb81df6c259bbc10
dist_disabled app 1913f0b38ab7b651dc40e11196dbfcd6eb1b894e8b76b689945975c18da46a06 N 9040 host9088 49998151ab1ab285
dist_drums app 0a95ab0cf20bffa7d494c49c29fa965644c9a7604cb8c7e08eda2142f51097e2 N 9088 host9088 same
drums_kit app 5c5d2740d423f9a6e69a0b7423ec78a2e0610835378a00914c869f9b447bdfe1 N 9056 host9088 f4250a0dfd038379
full_mix app a0209a74de1f6782f3955ce5be92d119792904404607883847f2d8c6ac34966c N 9088 host9088 same
loop_bar1 app 55206391fff1eecca72edb5241b163de88ab3334bc09c913f964e20b572749be N 9072 host9088 e9c5db3a309573df
loop_bar2 app f8d7db30e5d906681b9fc5ae96c4f38f7e4f5ab7f0b94244c31014aeb8f54c36 N 9088 host9088 same
loop_chase app f2c254c3efd8a8bf30eee66631fe01518952378d7b8eb51d2fb1e95a6ec48b04 N 9120 host9088 3f4e4f0104e51aa2
loop_events_inside app 81c9130fa77e4c61da8206a14bf377d615be07988a61ed57ade1f4732000a9b7 N 9104 host9088 efada817f7b941d9
mixer_delay_send_sweep app a527798d81bfd8704dbbc8bb68c7d2745e990ddfcf5b00eadbbecf31f67a7875 N 9104 host9088 7231dac0a549d0d1
mixer_dist_switch app 4c9751a6bafb4b4f60d223fb9afca92b7d2f76b530583f6b678a7376558c88ae N 9136 host9088 f96b161ff938da81
mixer_level_sweep_303 app eae1464e189722f76e88e5fa4aaa9e6ecd8e69f7ca372f7956eb4cfceb44a149 N 9136 host9088 9fcfa77b37029042
mixer_level_sweep_808 app 8257d686dea6c29b3866da5d7d60d98a69c87bcbc119bb75b3d4a635c1a04234 N 9120 host9088 60dda7d134cf6896
mixer_level_sweep_909 app 646c782f39079b2775084affe6cb3de3668224b3374c46c284e7d7862f26c9e3 N 9104 host9088 6f8ff059bb3ff532
mixer_master_full app 0ffaee122ebe848761baeee3cbecf896c6dbb81f934c306e7850c72f0c933df0 N 9088 host9088 same
mixer_master_low app 764ff57d42a059635f95e9d042dcceaafb2375598c373cefe6bda40a8c12877b N 9104 host9088 204c7293eb93ac0c
mixer_pan_sweep_303 app 538abcff76edbb82441099b9ada40fbf9834fe7eae8c863e65de9ba34e79ee61 N 9072 host9088 afe9620842c20c21
mixer_pan_sweep_808 app ea7ac0812033e8f2a0dc0de9a3606084b14933060ec80f659e09cb64e7dbc931 N 9088 host9088 same
mixer_pan_sweep_909 app 37e2d010ae789970e43b94015667aa75d3bbe85db714b6fdd46ae2f585fcd184 N 9104 host9088 f555929f90df3a5c
mixer_pcf_switch app 5defe493ae3aeb5105653f69ef85434d7491083199a19410da8149be5c31c8eb N 9120 host9088 27719b7021729cba
pcf_bp_freq10_res100 app b28f95a70765ade45cbb3d8fb568db5f73beeb1b095c98ecf4959d8f700dd57e N 9104 host9088 22c1fe4315626944
pcf_bp_freq120_res20 app e055f3fbfd74d8659f619aef32e2ff2dfe7191b8c5a44df4f6db4656ad3709c8 N 9104 host9088 4088948a00e15316
pcf_bp_wave0 app 689f23f372e3a7d4bfb5e9803c6168de65f072d84919969940cc9c01aa7c7479 N 9072 host9088 afadf57ddc6a1c2d
pcf_bp_wave127 app 0a90d04342bfc159bae46c1dfd351be1d646c6378b5d60b7aee3c722b48c38db N any host9088 same
pcf_bp_wave64 app c898db62df8952f048ecde2b827a80baa566fb054a7ecc5f22ce226531b6096e N 9072 host9088 4eea8597e37a6673
pcf_lp_amount0_decay64 app c25526c2a26a40bf9b569ab6063f4b03a2d13136c22784f2080e164069c794e7 N 9120 host9088 24089c04589609b2
pcf_lp_amount127_decay0 app 25584881a6e24605173eed1c6eaa76aa4893b3166d5b5545d918575baa05d709 N 9056 host9088 24089c04589609b2
pcf_lp_amount127_decay127 app 451c0e537f0480fe5836d0aaece951e34c5035aee83588e8dd22d00ff0521150 N 9104 host9088 24089c04589609b2
pcf_lp_freq10_res100 app 23152bd1f24da8bcb43856739fa95e65b05a7c4a1530c4d73ea0c48f1d750043 N 9056 host9088 6e46560a723c2b0c
pcf_lp_freq120_res20 app 745862b6d5a255bf8aa9a3fc7e53e95c989f82fbc1af405ac5c34a2050d14205 N 9088 host9088 same
pcf_lp_wave0 app e691094ae2e41b802c167ec59ff48213c34d2509105f6e40e6003bf08da446c5 N 9072 host9088 3b26c3b925b1bced
pcf_lp_wave127 app 0a90d04342bfc159bae46c1dfd351be1d646c6378b5d60b7aee3c722b48c38db N any host9088 same
pcf_lp_wave64 app 24089c04589609b222f5856c291045f0b1383c00f171d3ca273df3e845a4a5c5 N 9088 host9088 same
pcf_on_909 app 36c0139063f7385d3e3dc26b1e990ef290d5fcaf1e62100ec997320cf2071b89 N 9264 host9088 864a1941f96b6727
shuffle_303_g0 app 85325fa2a6944907df71b42bdb57b1a88a21217a5bcbfa9118719fb1c2eb1981 N 9104 host9088 12f252bcb4f0b9ba
shuffle_303_g127 app fb4a0ac9cd1516e7dfef701bad0fadf48b9615b201931a1c784ff6e4183fba75 N 9088 host9088 same
shuffle_303_g64 app 6c4367a53cfeaee18b8f82c833eacf7d30a893b037a7f6be82691a85c89207ef N 9104 host9088 b36c372e181243f8
shuffle_drums_g127 app b410d100e1fc8677941f04d7ef578b4aab7ab06f410c6d37d41c1b8c1e41cdc1 N 9088 host9088 same
shuffle_drums_g64 app 8732d3542b64ff79975f6d765459f8d32d2f5835ea6bd799dc6169d9dab72352 N 9120 host9088 9d16176aa0463bb9
shuffle_flag_off app 85325fa2a6944907df71b42bdb57b1a88a21217a5bcbfa9118719fb1c2eb1981 N 9104 host9088 12f252bcb4f0b9ba
silence app 0a90d04342bfc159bae46c1dfd351be1d646c6378b5d60b7aee3c722b48c38db N any host9088 same
tempo_143_21 app 63c85fd17ab5579d1dad9a686ee7c760b8e438a935df08447485997f2878566a N 9104 host9088 d2818dfa2fc103b3
tempo_200_003 app 7f91c01bd798e5f6ef0a5eb479fbb139a319ce2fc47cb779f6760c0f7a3ef642 N 9136 host9088 2d3c2e5aedfd39f4
tempo_66_667 app c8f40d95d3e2f84c63b09574dbd0146745e0aa3465638e7cc7cc9db7ef948470 N 9072 host9088 8c0814ad5139a82f
tempo_97_3 app 8565a4cc7ff7e685a87653b725e77d3e7fddbf4ab0b38b682c490d980281a312 N 9104 host9088 add13d00188d2230

AC check: #1 all 145 diagnostic songs exported by the app (fresh process each, the 11.5 protocol) and rendered by rbhost (N = 9088 and the 49-value sweep), compared with rbcmp.py / sample equality; the table is docs/export_state.md section 13 (145 rows). #2 every pair matches byte for byte at a fitted N; the only differences (at a fixed N) are explained by N (303 phase / 303 idle output); no rbhost fix was needed, so no fix commit to reference. Follow-up: task-039 (PCF wave 127 songs export silence, in the app too).

Review round 1 (send back, docs only; the evidence held):
1. The N - N_t = 47.6 +- 14.6 relation is now marked [G] and described as an empirical fit over 142 points (spread 3..82), with no mechanism attributed.
2. Corrected: the app ran as 7 rb_export.py invocations, not 6: the 3-song first run (app_b0: silence, 303_accent0, 808_snare) plus batches of 24, 24, 24, 24, 24 and 22. Step 2 of the work log above was corrected in place in round 2.
3. pcf_on_909 is no longer called an outlier in N - N_t (its value is 47, inside the spread). Only its open-to-export time, 13.38 s against 13.08..13.21 s, stands out.
4. export_state 11.2 and initial_investigation: 'a fresh export depends on the song and N only' is marked [C] with its basis (145 songs, task-012: 142 unique fits plus 3 N-independent silent songs, one process per song, the 11.5 protocol). It stays [G] for other songs and protocols.
5. Section 13 now carries [C]/[G] marks. The 303-notes split was checked against the .rbs files (scratch split.py, tools/rbs.py to_json; a song has notes when a TB-303 pattern holds a non-rest step within its length). 60 songs have notes and 85 do not. Cross-tab at N = 9088: notes and a large diff 43; notes and equal 17; no notes and +-1 LSB 55; no notes and equal 30. There is no other combination.

Review round 2 (send back, task notes only; the docs were accepted). The defects: work-log step 2 still said 6 invocations, and step 5 still called pcf_on_909 the outlier, which contradicted the reworded section 13. The reviewer also saw no record of round 1 in the committed file. Fixes: steps 2 and 5 were corrected in place through the backlog CLI (--notes with the edited text). The round 1 record above is in the amended commit, and so is this round 2 record.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
rbhost reproduces the original app's export byte for byte on all 145 diagnostic songs. App: tools/rb_export.py --restore-prf --startup-sound off --wait-after-open 10, one fresh process per song, all 145 captured (62 min). rbhost: tools/rbhost/run.py (7838f84, unchanged), every song at N = 9088 and swept over N = 8704..9472 in steps of 16. For every song one N gives a WAV whose sha256 equals the app's: 142 songs match exactly one N (fitted N 9040..9152, one at 9264); silence and the two PCF wave-127 songs (all zeros in the app too) match every N. At the fixed N = 9088, 47 songs are equal and 98 differ by N only: 43 songs with 303 notes from the first note (303 phase), 55 without by +-1 LSB (303 idle output). The expectation that songs without 303 notes match at any N is refuted: the idle 303 output ties every non-silent export to N. N tracks the runner's timeline: N - (t_export - t_open)*44100/64 = 47.6 +- 14.6 blocks, so a 7-render sweep seeded there suffices (input for task-038). Docs: docs/export_state.md section 13 (table) and the [G] of 11.2 confirmed; docs/initial_investigation.md protocol paragraph updated. Follow-up: task-039. Branch worktree-agent-a7d4f8be649280156, one commit. Tests: the tool runs above (no code changed).
<!-- SECTION:FINAL_SUMMARY:END -->
