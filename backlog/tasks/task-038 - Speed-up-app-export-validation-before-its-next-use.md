---
id: TASK-038
title: Speed up app-export validation before its next use
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-26 15:08'
updated_date: '2026-10-26 15:55'
labels:
  - tools
  - testmachine
milestone: m-2
dependencies: []
references:
  - docs/export_state.md
  - tools/rb_export.py
  - tools/rbhost/README.md
priority: medium
type: enhancement
ordinal: 30500
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Validating rbhost against the original app (task-012: app exports with tools/rb_export.py, then an idle-count fit per export with tools/rbhost/run.py --idle-sweep ... --match) took a long time, mostly waiting. App exports are needed again only occasionally: the task-013 cross-check set, after a change to rbhost's .rbs-to-message conversion, and for new input paths such as mods (task-031). The user's rule (2026-10-26): the next time this process runs, optimize it first. Do this task before any further bulk app-export validation.

Start from measurements: take the timings of the task-012 runs (its notes, the rb_export.py sidecars) and break one export and one fit down by phase (app start, MIDI box, open, wait after open, export dialog, render, file-complete polling, download, quit; for the fit: upload, process start, per-candidate render, compare). Then cut the waits and serialize less:
- rb_export.py: shorten fixed sleeps to what the app needs (poll for the window or file state instead of sleeping; shorter poll intervals; measure the minimum safe wait after open: with N fitted afterwards, the protocol's 10 s wait need not be long, but check that shorter waits keep the exports reproducible as docs/export_state.md section 11 requires, and update that section if the protocol changes).
- Fitting: run candidates in parallel (several rbhost processes, one permit each, up to the free permits; or many candidates inside one rbhost process, reusing the loaded DLL and song); estimate N first from the signal (the 303 phase method of tools/experiments/task010/shift.py and phase.py) and only verify a few candidates near it; stop at the first exact match; fit all songs of one app session together when they share N.
- General: upload rbhost and its inputs once per batch; render a batch of songs per rbhost process; compare on the machine or stream hashes instead of downloading every WAV when only equality matters.
Keep the results identical: re-run a sample of task-012's comparisons with the optimized tools and show the same verdicts and N.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The time per app export and per N fit is measured before and after, broken down by phase, in the task's notes
- [ ] #2 rb_export.py waits are cut (polling instead of fixed sleeps where possible) and the fit runs candidates in parallel and/or seeds them from a signal estimate of N
- [ ] #3 A sample of task-012's comparisons re-run with the optimized tools gives the same verdicts and N; docs/export_state.md reflects any protocol change
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-26: inputs from task-012 and a timing run.
- N from the runner's timeline (docs/export_state.md section 13, [G] empirical fit over 142 songs): N - round((export started - song opened) * 44100/64) = 47.6 +- 14.6 blocks (range 3..82), so a window of about 7 candidate renders around that estimate would have found every task-012 fit; the sidecars of tools/rb_export.py hold the timestamps.
- task-012's full run: 145 app exports in about 62 min (7 invocations), rbhost sweeps of 49 N values per song.
- rbhost on a full song (Calm, 14,003,446 frames = 5.3 min), measured 2026-10-26 by hand on the machine: upload of rbhost.exe + message file 0.34 s; render 2.52 s at 0 idle blocks, 2.69 s at 9,088 idle blocks (about 125x realtime; idle blocks cost ~0.17 s per 9,088); download of the 56 MB WAV 2.1 s. So about half of a full-song render is the download: compare hashes on the machine.
<!-- SECTION:NOTES:END -->
