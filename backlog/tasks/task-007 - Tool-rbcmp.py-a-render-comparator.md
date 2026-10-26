---
id: TASK-007
title: 'Tool: rbcmp.py, a render comparator'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
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
- [ ] #1 rbcmp.py reports equality, first difference, max/RMS difference, the histogram and the best alignment offset, with --json and the exit status described
- [ ] #2 tools/rbcmp_test.py passes and covers equal files, a 1-LSB difference, an offset copy and different lengths
<!-- AC:END -->
