---
id: TASK-014
title: Capture and commit the reference corpus
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 16:05'
labels:
  - tests
  - testmachine
milestone: m-0
dependencies:
  - TASK-009
  - TASK-013
  - TASK-006
  - TASK-007
  - TASK-011
  - TASK-012
priority: high
type: task
ordinal: 14000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Capture the reference corpus: every diagnostic song (and the shipped songs, as the storage decision allows) rendered by the decided protocol, stored per the storage decision, with a MANIFEST.md listing for each reference: the song file and its SHA-256, the protocol and tool (rbhost or rb_export.py, with versions or commits), the capture date, the WAV's SHA-256 and length in frames. A script re-captures the whole corpus from the manifest; rbcmp.py checks a fresh capture of three songs against the committed references.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The corpus and its MANIFEST.md are committed per the storage decision
- [x] #2 The re-capture script exists and a fresh capture of three songs equals the committed references (rbcmp output in the notes)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
tools/rbcorpus.py (capture/capture-app/recapture/check-flac/manifest); tests/refs/manifest.json is the source, MANIFEST.md generated. diag: 145 rbhost renders at --idle-blocks 9088 as FLAC. shipped: sha256+frames only. app: 11 app exports (FLAC) with the fitted N from task-012. Verify: FLAC decode == WAV sha; fresh re-render of 3 diag, all app at their N, 2 shipped.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-26: per the task-013 decision the corpus is rbhost renders (checked by a smaller app-export set under the docs/export_state.md section 11.5 protocol), so this task depends on rbhost (task-011) and its validation (task-012).

Branch: worktree-agent-a02f23aa78a00bf55

Captured with tools/rbcorpus.py capture diag|shipped (rbhost.exe sha256 13814eae2ebe, rebuilt here; tools/rbhost commit 7838f84) and capture-app (app WAVs/fits from task-012's scratch: no app re-capture). Corpus: diag 145 FLAC (12 MB), app 11 FLAC (0.99 MB), shipped 8 hash-only, total tests/refs 13.9 MB (budget 50 MB). Shipped: only 8 of the 16 non-default songs use 'Standard ReBirth'; rbhost refuses the 8 with another mod (Pitch Black Edition 2.0, Massen's MSM 2.0, Metallicon+; rbsmsg.mod_id), so they wait for RBM work (task-031); listed in MANIFEST.md. App set: 303_accent0 (N 9088), 303_vintage_saw 9072, 808_snare 9040, 909_snare 9072, dist_a100_s64 9072, delay_straight_fb64 9088, comp_ch_a30_t64 9072, pcf_lp_freq10_res100 9056, tempo_143_21 9104, full_mix 9088, shuffle_303_g64 9104. Verification run (ver.sh): check-flac: 156 FLACs decode to their manifest WAV sha256, 0 not. recapture diag 303_accent0 full_mix pcf_on_909: equal sha and rbcmp exit 0 each. recapture app (all 11, at their N): equal, rbcmp exit 0 each. recapture shipped Calm and 'A Taste of Haste': sha256+frames equal. Note diag 303_accent0 wav sha 1efcc132... equals the app's, as task-012 found at N 9088.

Review round 1 fixes: (1) app entries now carry tool_commit efcae85 (git log of tools/rb_export.py, older than the 15:53 captures) via git_commit(path); (2) 'captured' for app entries is the date of the export from the task-012 sidecar (machine.time, 2026-10-26), for new captures today; (4) rbcmp output of the 3 fresh diag re-renders (committed FLAC decoded = A, fresh rbhost render at N 9088 = B): 303_accent0: 'A: 176400 frames, 2 ch, 44100 Hz / B: 176400 frames, 2 ch, 44100 Hz / EQUAL', exit 0; full_mix: 325661 frames both, EQUAL, exit 0; pcf_on_909: 176400 frames both, EQUAL, exit 0. MANIFEST.md regenerated. The commit message now has Decision: task-006 and task-013 trailers.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
tools/rbcorpus.py plus tests/refs/ (manifest.json, generated MANIFEST.md, 145 diag FLACs, 11 app FLACs with fitted N, 8 shipped songs hash-only), all rbhost at --idle-blocks 9088 (diag/shipped). 13.9 MB of the 50 MB budget. Verified: FLAC decode exact (156/156), fresh re-render of 3 diag + 11 app (at N) + 2 shipped equal, rbcmp exit 0. Not captured: 8 shipped songs with non-standard mods (need RBM, task-031). Testing: tool run on real songs, no cargo.
<!-- SECTION:FINAL_SUMMARY:END -->
