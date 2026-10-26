---
id: TASK-019
title: 'Engine tables, generated and checked against the original''s dumps'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 17:28'
labels:
  - engine
milestone: m-1
dependencies:
  - TASK-048
priority: high
type: feature
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The engine's lookup tables: the ones computed at init (T and PAN, 303 filter table FILT and pitch table, PCF LP/BP tables, distortion waveshaper W, T808, T909, DT, the cutoff-smoother and PCF constants) and the static ones read from the DLL's data (distortion grid, PCF patterns/lengths/resolutions/decays, 808/909 tone, decay and attack tables, SEQ808/SEQ909, 303 tune table). Formulas and addresses: docs/initial_investigation.md sections 4, 6, 7, 8. Generate the computed tables in Rust (f64, f32 stores, the CRT pow and x87 transcendentals approximated by Rust's) and compare each against rbhost's --dump-tables output in a test; where an entry differs, find whether a faithful x87/CRT emulation fixes it, else embed the dumped table (as a generated Rust source with a comment saying why). Static tables: read their bytes from original/ReBirth.dll in a test and assert the Rust constants equal them.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every table is bit-identical to the original's dump (f32 bits), checked by cargo test
- [x] #2 Each embedded (not generated) table has its reason in a comment and in the task's notes
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Dump the run-time tables once with tools/rbhost/run.py --dump-tables (one permit); add the two static-init smoother constant triples (0x1012470C, 0x10100260) to rbhost's TABLES so they are dumped too.
2. Read every builder in the disassembly (capstone over original/ReBirth.dll plus idalib): sub_100046B0, sub_10023920, sub_100234A0, sub_100172C4, sub_10017574, sub_10011210, the 808/909 ctors, the static inits sub_10026BB0/sub_10017840; decode their constants from the file bytes.
3. Prototype in Python with plain f64 libm, diff against the dumps, then fix formula/constant errors; only if entries still differ try x87/CRT emulation, then embedding.
4. Rust: crates/rb-engine/src/tables/ (mod.rs generators, data.rs static tables from the DLL). Commit the dumps as fixtures in crates/rb-engine/tests/fixtures/tables/ with manifest and hashes; tests/tables.rs compares every generated table bit for bit and every static table and builder constant against original/ReBirth.dll by file offset.
5. Docs: corrections and new addresses into docs/initial_investigation.md sections 4, 6, 7, 8, 10.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a734adc1e6f8c6bea

Dump: tools/rbhost/run.py --dump-tables -o SCRATCH/dump tests/songs/diag/songs/303_accent0.rbs (one permit, 2 s). Remote C:/ReBirth RB-338/ReBirth.dll sha256 4c3a6441a2cd50f797d99b1c5b6cf0aaf198c7ad8ca2648196b530c2e3e58b12 = original/ReBirth.dll. Second run with rbhost.c TABLES extended by smooth303 (0x1012470C, 12 bytes) and smooth_pcf (0x10100260, 12 bytes): rbhost sha256 f397f2cd2aa43ba12d47179748de46a7d02a56f4b45e86901e6dea8b1920de38; the ten earlier .bin files are identical between the runs.

Static data lives in .data1 (VA 0x1014F000, file 0x33000) and _data (0x10156000, file 0x38A00) too, not only .data below 0x10034400: e.g. the T step f32(1/24) 0x1014F8E4, every builder constant 0x10151xxx..0x10154xxx. The static-init list is at 0x10030010..0x10030060 (sub_10003740, sub_10017840, sub_1001A920, sub_10021CD0 param ranges, sub_10026BB0).

Python prototype (plain f64, macOS libm pow/exp/cos/sin, the exact operand order of the disassembly): every table bit-identical at the first try except dist W (7780 of 8192 differ by 1-2 ulps). Cause: the builder's f32 constants are not f32(pi/8), f32(pi/1024): 0x3EC90FD0 and 0x3B490FD0 (pi truncated), and 1/961 is 0x3A88642A. With the file's constants W is exact too. Margin: of the 71783 f32 stores the closest f64 value is 1358 f64 ulps from an f32 rounding midpoint, so libm errors of a few ulps cannot change a stored f32. No x87/CRT emulation was needed (CRT pow 0x1002DDF0 is a table-driven x87 pow; under PC=53 its fmul/fadd round to f64).

Builders read in the disassembly (capstone over original/ReBirth.dll for explicit operand sizes, idalib for xrefs): T/PAN sub_100046B0; pitch sub_10023920 (cube by squaring, 55*2, 1/44100 at 0x10153AB0, pow(p,3072) divisor via fdivrp DE F1); FILT sub_100234A0 (all locals qword; c2 by fdivp DE FA = st2/st0); PCF LP sub_100172C4 / BP sub_10017574 (index scale 1/512, LP num (c1+(1-c0))*2f, BP (cos(W*pi)*sqrt((1f-c0)+c1))*3f); dist W sub_10011210 (W[t][e] then W[t][256-e] = -v, so W[t][128] = -0.0); T808/T909 in the 808/909 ctors (pow(2,1/192|1/144), 64th power by six squarings, fistp truncation); DT (x in f64 register, 2.5e-5 at 0x101542A8); static inits sub_10026BB0 / sub_10017840 (identical code and constants; stores b2, A, g).

Static tables found by their indexed loads in the drum code: 808 SD slots i16[5] 0x10034090, SEQ808 0x1003409C, noise slots u16[4] 0x100340DC, TONE8 f32[8] 0x100340E4 (0x1002A171), DEC16 f32[16] 0x10034104 (0x1002A18A); 909 SD slots 0x10034210, SEQ909 0x1003421C, SDN slots 0x1003425C (9,10,11,0), C8 f32[8] 0x10034264 (0x1002DBC8). TB303 tune table 0x10033F90 is i16[32].

Rust: crates/rb-engine/src/tables/mod.rs (generators, Tables::new()), data.rs (16 static tables, generated once from the DLL by a scratch script and asserted byte-equal in the tests), tests.rs (11 unit tests: every dump bit for bit with a count/ulps report on mismatch; static tables and ~110 builder constants, data and mov immediates, against the DLL by file offset). Fixtures: crates/rb-engine/tests/fixtures/tables/ (12 .bin, manifest.json, README.md with the hashes).

Per table, generated-and-exact (0 differing entries): T 128, PAN 32768, pitch303 6144, FILT 32768, PCF LP 12288, PCF BP 12288, dist W 8192, T808 128, T909 128, DT 16, 303 smoother 3, PCF smoother 3. Embedded: none. Emulation: not needed (CRT pow and x87 transcendentals approximated by std f64 powf/exp/cos/sin).

Fragility: T808[64] = trunc(pow(s,64)*(256/s^64)) is exactly 256.0 because macOS pow(s,64) equals six squarings bit for bit; T909[64] is 256 + 6 f64 ulps. Experiment: the same code with the pure-Rust libm crate 0.2.16 (scratch crate, libmtry.sh) passes all 11 tests too. The platform question (std uses the platform libm) is task-048 (Decide, @pingw33n), recommending the libm crate.

Tests: cargo test 45 pass (11 new), also --release; cargo clippy --all-targets -D warnings clean; rbhost Python tests 15 pass. No corpus run: no engine output change yet (the tables are not wired into rendering; task-021 onwards use them).

AC1: tables::tests::{level_curve,pan,tb303_pitch,tb303_filter,pcf,dist_shaper,drum_tables,smoothers}_matches_dump assert f32 bits (u16 for T808/T909) equal the rbhost dumps; all pass (cargo test -p rb-engine --lib tables: 11 passed). The static tables are asserted byte-equal to original/ReBirth.dll (static_tables_match_dll), and the builders' constants too (builder_constants_match_dll).
AC2: no table is embedded (every computed table is generated and exact), so there is no embedded table needing a reason; recorded in the module doc of crates/rb-engine/src/tables/mod.rs and here.

Review round 1 (2026-10-26): engine work verified by the reviewer; defects addressed:
- Decision task-048, decided by the user 2026-10-26 (option 2): the generators now use libm =0.2.16 (pow, exp, cos, sin) instead of std's powf/exp/cos/sin; rb-engine depends on libm. All 12 table tests pass, debug and --release, still bit-identical. task-048 closed Done, so task-019's dependency on it is satisfied.
- The f32 store margin is now a test: tables::tests::store_margins records every f32 store through st() in mod.rs and asserts the minimum >= 1358 f64 ulps. Per table (f64 ulps): t 6163976, pitch303 429377, filt303 1625, pcf_lp 1358, pcf_bp 1358, dist_w 80480, dt909 23622313, smoothers 4923383.
- data.rs provenance: the generator is committed as tools/gen_static_tables.py (reads original/ReBirth.dll by VA through the section map, runs rustfmt); its output equals the committed data.rs except the header, which now names the script and the test static_tables_match_dll.
- docs/initial_investigation.md section 10 item 4 and section 4 now name libm, the margins and the test.
Tests: cargo test 46 pass (12 tables), clippy -D warnings clean, fmt; tools/gen_static_tables.py run and diffed against data.rs.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Every engine table is generated in Rust and bit-identical to the original's run-time dump; none is embedded and no x87/CRT emulation was needed.

- crates/rb-engine/src/tables/mod.rs: generators following the disassembly operand by operand (f64, f32 stores, truncation) for T, PAN, 303 pitch, 303 FILT, PCF LP/BP, dist W, T808, T909, DT and the 303/PCF cutoff smoothers; Tables::new(). Transcendentals from the pure-Rust libm crate =0.2.16 (Decision task-048), so the tables are platform-independent.
- crates/rb-engine/src/tables/data.rs: 16 static tables from the DLL, generated by tools/gen_static_tables.py.
- crates/rb-engine/src/tables/tests.rs: 12 tests; dumps compared bit for bit, the f32 rounding margins (min 1358 f64 ulps) asserted, static tables and ~110 builder constants compared with original/ReBirth.dll by file offset.
- Fixtures: crates/rb-engine/tests/fixtures/tables (rbhost --dump-tables of 303_accent0.rbs, manifest, hashes). tools/rbhost/rbhost.c dumps the two smoother triples too.
- Result per table: all 12 generated-and-exact (0 differing entries of 104,854). The one fix on the way: dist W's pi/8 and pi/1024 constants are 0x3EC90FD0 / 0x3B490FD0 (pi cut short), not f32(pi/8); with f32(pi/8) 7780 of 8192 entries were 1-2 ulps off.
- Docs: docs/initial_investigation.md sections 4, 6.1, 6.2, 7.3, 8.1, 8.2 and 10 item 4 updated with the exact builders, the static table addresses and the outcome.
- Review round 1: libm applied (task-048 Done), margin test and data.rs generator committed.
- Tests: cargo test 46 pass (also --release for the tables), clippy clean, rbhost Python tests 15 pass. No corpus run: rendering does not use the tables yet.
- Branch worktree-agent-a734adc1e6f8c6bea, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->
