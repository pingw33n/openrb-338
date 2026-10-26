---
id: TASK-020
title: Built-in drum samples loader
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 20:09'
labels:
  - engine
milestone: m-1
dependencies:
  - TASK-015
  - TASK-016
priority: medium
type: feature
ordinal: 23800
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Load the built-in drum samples as the decision on their source says: parse the PE resource directory of ReBirth.dll (type "AIFF", IDs 1000..1045, the ID-to-instrument map in docs/initial_investigation.md section 7.3/7.4), parse AIFF (COMM, SSND, 16-bit big-endian, channel 0 only, sample rate ignored), keep raw i16 with 512 zero samples of padding as CWaveSample does. Tests: all 46 samples load from original/ReBirth.dll with the expected frame counts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All 46 built-in samples load per the decision, with frame counts checked in cargo test
- [x] #2 The AIFF parser rejects non-16-bit and missing-chunk files with clear errors (tested)
- [x] #3 The rb CLI takes --rebirth-dll PATH (and a default location) to load the samples, and errors clearly when drums are needed but no ReBirth.dll is found
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Check against ReBirth.dll.i64 (idalib, scratch copy): resource lookup, AIFF/COMM/SSND parse, CWaveSample padding. 2. rb-engine: samples.rs + samples/pe.rs + samples/aiff.rs (SampleSet::from_dll). 3. CLI: --rebirth-dll, RB_REBIRTH_DLL, search, errors; load only when a drum machine plays. 4. Tests (engine + CLI), expected frames and sums from an independent Python reader. 5. docs/initial_investigation.md 7.4.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-26, per the user: runs just before task-024 (the first task that needs samples), after the CLI (task-028). This task also adds the sample source to the rb CLI per the task-016 decision: `--rebirth-dll PATH` (and a default location), with a clear error when a song needs drum samples and no ReBirth.dll is found; 303-only rendering keeps working without it.

Branch: worktree-agent-a05bd0eabb93a9d8f

IDA (copy of ida/ReBirth.dll.i64, headless, read-only, closed save=false): FindResourceA only at 0x1000B663 (sub_1000B620, type string, 16-bit id) and 0x1000D546; table builder sub_1001A030 (slots 1..24=1000..1023, 25..45=1025..1045, 46=1024); AIFF chain sub_10003300 -> sub_10003170 (COMM) -> sub_10002FC4 (SSND, mono read + swap, or sub_10009DD0/sub_100099DC channel 0 de-interleave); CWaveSample ctor sub_1000A9C0: 2*frames+1024 bytes zeroed, so the 512 zero samples FOLLOW the audio (the audio starts at 0). Non-16-bit throws XUnsupportedFormat. SSND offset is honored (1034..1036 have offset 458). All in docs/initial_investigation.md 7.4. Independent check: /usr/bin/python3 reader (own PE+AIFF parser) gave frames and sample sums for all 46; the test holds both. Not confirmed: whether FORM type 'AIFC' is rejected by sub_100054D4 (I require FORM+AIFF; all 46 pass); the multi-language case (none in the 2.01 DLL).

Design: engine API rb_engine::samples::SampleSet::from_dll(&[u8]) (+ Sample::from_aiff); the CLI finds and reads the file. Validity check = the resource set (all 46 IDs present and valid AIFF), not a SHA-256: it checks what the engine needs and also accepts a patched/repacked 2.01 DLL; the exe test shows a PE without AIFF gives NoAiffResources. Search order: --rebirth-dll (must exist), RB_REBIRTH_DLL (must exist), ReBirth.dll next to the rb binary, then on Windows %ProgramFiles(x86)%/%ProgramFiles% Propellerhead/ReBirth and ReBirth RB-338 plus C:\\Program Files (x86)\\Propellerhead\\ReBirth and C:\\ReBirth RB-338. Explicit choices are never second-guessed by falling through; implicit ones are for convenience, with the binary's folder first because a file put there is a deliberate act. Samples are loaded only when a drum machine plays (drums_playing, the former 'drum plays notes' test); the CLI holds the SampleSet unused until task-024 adds it to Devices (task-024 should pass it; I did not touch Engine/Devices).

Review round 1 (merge verdict, small fixes): docs 7.4 marks the FORM-header/AIFC part [G] and says rb requires FORM+AIFF; aiff.rs module comment now cites the same addresses as the doc (COMM found by 0x10003170, fields 0x10001674, SSND 0x10002FC4 with header 0x10001774); pe.rs skips numeric IDs above 0xFFFF instead of truncating (no test: needs a synthetic PE, not cheap); docs state the drum voices read i16 (frames+512).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Built-in drum samples now load at run time from the user's ReBirth.dll (task-016 option 1), no Windows API. rb_engine::samples (samples.rs, samples/pe.rs, samples/aiff.rs): SampleSet::from_dll parses the PE resource tree for type "AIFF", IDs 1000..1045, and the AIFF (16-bit, channel 0, SSND offset, 512 zero samples after the audio, as CWaveSample). Facts and addresses are in docs/initial_investigation.md 7.4. CLI: rb render --rebirth-dll PATH, RB_REBIRTH_DLL, ReBirth.dll next to the binary, Windows install paths; loaded only when a drum machine plays; clear errors (not found, not a ReBirth DLL with the samples). Tests: cargo test 108 pass (6 new engine in tests/samples.rs: 46 frame counts and sums derived by an independent Python reader, SSND offset, AIFF 8/24-bit and missing-chunk errors, non-PE, truncated PE, Rebirth.exe with no AIFF resources; 3 new CLI in tests/rebirth_dll.rs). clippy -D warnings and fmt clean. Branch worktree-agent-a05bd0eabb93a9d8f, one commit. Follow-up for task-024: pass the SampleSet to the drum devices (the CLI loads it and holds it unused).
<!-- SECTION:FINAL_SUMMARY:END -->
