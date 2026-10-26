---
id: TASK-006
title: 'Decide: where test songs and reference renders are stored'
status: Done
assignee:
  - '@pingw33n'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 09:21'
labels:
  - decision
  - tests
milestone: m-0
dependencies: []
priority: medium
type: spike
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The project needs test songs (.rbs, small) and reference renders of them (WAV from the original, 44.1 kHz 16-bit stereo; a few bars each, but a full demo song is ~30 MB per 3 minutes as WAV). Where do they live?

Options:
1. In the repository: songs under tests/songs/, references as FLAC (lossless, about half the size) under tests/refs/, with a MANIFEST.md (song, its SHA-256, the capture protocol, the tool version, the WAV's SHA-256 before compression). Keep songs short so the total stays under ~50 MB.
2. Git LFS for the references.
3. Outside git (a folder on the Mac, ~/rb-refs), with only the manifest and hashes in the repository.

Recommendation: option 1, with a size budget; long songs (the shipped demos) only as hashes plus a script to re-capture them. Also decide whether the shipped .rbs songs (Propellerhead's) may be committed.
<!-- SECTION:DESCRIPTION:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Decided by @pingw33n on 2026-10-26: option 1, as recommended. Test songs go in the repository under tests/songs/; references go as FLAC (lossless) under tests/refs/ with a MANIFEST.md (song, its SHA-256, the capture protocol, the tool version, the WAV's SHA-256 before compression). Songs are kept short so the total stays under about 50 MB; long songs (the shipped demos) are kept as hashes plus a script to re-capture them rather than as committed renders. The .rbs songs shipped with ReBirth (Default Songs/, Demo Songs/) may be committed.
<!-- SECTION:FINAL_SUMMARY:END -->
