---
id: TASK-007
title: 'Tool: rbcmp.py, a render comparator'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 10:53'
labels:
  - tools
  - tests
milestone: m-0
dependencies: []
priority: high
type: feature
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Every comparison between the original's render and ours (and between two renders of the original) needs one tool. Write `tools/rbcmp.py` (stdlib Python; FLAC input through the `flac` command when installed): compare two 16-bit stereo renders and report exact equality, the first differing frame (as frame index, time, bar/beat at a given tempo), per-channel max absolute difference, count of differing samples, RMS difference, the differences histogram (|d| = 1, 2..3, 4..15, more), and an optional search for a constant frame offset (`--align N`: best offset within ±N frames). Exit status 0 when equal or within `--tolerance LSB`, 1 otherwise; `--json` for machine output. Tests in `tools/rbcmp_test.py` on generated files.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 rbcmp.py reports equality, first difference, max/RMS difference, the histogram and the best alignment offset, with --json and the exit status described
- [x] #2 tools/rbcmp_test.py passes and covers equal files, a 1-LSB difference, an offset copy and different lengths
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Single stdlib tool tools/rbcmp.py (WAV parser, FLAC via flac -d -c), tests on generated WAV/FLAC in tools/rbcmp_test.py.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a78404c12b488711d

flac installed (/opt/homebrew/bin/flac); FLAC test ran and passed (skipped automatically when flac is missing). Align: window of 32768 frames from first non-silent frame, offset k means A[i]==B[i+k]; full stats re-reported at the best offset. Exit 2 on read error (extra). Length mismatch always exits 1, even with --tolerance. 8 tests pass: /usr/bin/python3 -m unittest tools/rbcmp_test.py

Review round 1: (1) parse_wav now raises ValueError on a truncated fmt chunk and on 0 channels (exit 2), test_malformed_wav added. (2) --align window_mismatches now counts frames (either channel differs), test_align_counts_frames added. 10 tests pass. Real render calm_export.wav (14003446 frames): self-compare EQUAL rc=0 0.15 s; one-LSB-flipped copy: first diff frame 7001717 (bar 80 beat 2 sixteenth 3 at 120 bpm), 1 differing sample, rc=1, 2.0 s; with --align 20 best offset 0 (0/32768), 1.9 s.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
tools/rbcmp.py and tools/rbcmp_test.py added on branch worktree-agent-a78404c12b488711d (one commit). Reports equality, first diff (frame/time/bar-beat-sixteenth at --bpm, default 120), per-channel max, count, RMS, histogram, --align N, --json; exit 0/1 (2 on read error). 8 unittest tests pass incl. FLAC.
<!-- SECTION:FINAL_SUMMARY:END -->
