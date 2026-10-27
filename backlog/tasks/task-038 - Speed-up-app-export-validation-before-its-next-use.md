---
id: TASK-038
title: Speed up app-export validation before its next use
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-26 15:08'
updated_date: '2026-10-27 05:47'
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
- [x] #1 The time per app export and per N fit is measured before and after, broken down by phase, in the task's notes
- [x] #2 rb_export.py waits are cut (polling instead of fixed sleeps where possible) and the fit runs candidates in parallel and/or seeds them from a signal estimate of N
- [x] #3 A sample of task-012's comparisons re-run with the optimized tools gives the same verdicts and N; docs/export_state.md reflects any protocol change
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Measure one app export and one fit by phase (rb_export.py timeline; run.py sweep). 2. rb_export.py: poll for the MIDI box and the main window instead of sleeping, drop the fixed 1.5/1.0 s sleeps, shorter poll intervals and stable time, one upload batch, hash check of the download, default T lower after measuring T = 10, 2, 0 on 8 songs. 3. New tools/rbhost/fit.py: seeded candidate order from the sidecar timeline, parallel rbhost processes (rbhost --private-port), hash compare on the machine, one upload batch; run.py --match-hash. 4. Re-run 8 songs (303 2.0 and Vintage, 808, 909, delay, odd tempo, compressor, pcf_on_909) with the new tools and compare with the serial run.py sweep. 5. Docs: export_state.md 11.6, README, test_machine.md.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-26: inputs from task-012 and a timing run.
- N from the runner's timeline (docs/export_state.md section 13, [G] empirical fit over 142 songs): N - round((export started - song opened) * 44100/64) = 47.6 +- 14.6 blocks (range 3..82), so a window of about 7 candidate renders around that estimate would have found every task-012 fit; the sidecars of tools/rb_export.py hold the timestamps.
- task-012's full run: 145 app exports in about 62 min (7 invocations), rbhost sweeps of 49 N values per song.
- rbhost on a full song (Calm, 14,003,446 frames = 5.3 min), measured 2026-10-26 by hand on the machine: upload of rbhost.exe + message file 0.34 s; render 2.52 s at 0 idle blocks, 2.69 s at 9,088 idle blocks (about 125x realtime; idle blocks cost ~0.17 s per 9,088); download of the 56 MB WAV 2.1 s. So about half of a full-song render is the download: compare hashes on the machine.

Branch: worktree-agent-a4af81832aa319040

Measurements (2026-10-27, machine free, scratch t038/ in the session scratchpad; run outputs under C:\re\rb\t038 and rb\rbhost\fit_*, cleaned up afterwards).

BEFORE (the committed tools): rb_export.py on 303_slide + 909_clap, T = 10: 50.9 s for 2 songs, 25.4 s per song. Phases of one export (sidecar timeline, means): prefs 0.05, app start 0.9, MIDI box sleep 4.0 + 1.0 (dismissed at +4.1), open 2.5 (incl. a 1.5 s sleep), wait after open 11.5, export dialog 0.8, export started 0.8, file complete 2.1 (incl. the 1.5 s stable wait), 1.0 s sleep for the progress box, download 0.1, taskkill ~1.5. One fit, 303_slide with run.py --idle-sweep 8704:9488:16 --match: 27 renders to the match at 9120 in 11.4 s = 0.42 s per render (rbhost 0.2 s of it, the rest RCS exec/poll at 0.2 s, tasklist per render, WAV download, local compare); the full 49-value sweep of task-012 = 36 s per song. RCS round trip 16-20 ms (info, list, hash), exec of a command 70 ms.

AFTER, rb_export.py: per song 9.6 s at the new default T = 2 (76.8 s for 8 songs), 7.8 s at T = 0 (62 s), 17.6 s at T = 10 with the new polls (141 s). Table of phases in docs/export_state.md 11.6. What changed: the MIDI box is found by polling the frame window's screenshot size (45814 bytes box vs 1739010 main window; same title, so the title alone cannot tell them apart: experiment exp1.py, polled every 0.15 s from process start: the box exists at 0.89 s and the main window appears only after Enter); no sleeps after the box, the open dialog, the progress box; stable time 1.5 -> 0.4 s; polls 0.05-0.15 s; songs uploaded in one batch; after the download the machine's hash must equal the file's.

AFTER, fitting (tools/rbhost/fit.py): 0.2-0.6 s per song to the first match (2-6 renders of 0.2 s) instead of 7-8 s (serial sweep to the match) or 36 s (49-value sweep); 8 songs in 4.1-4.3 s including build, permit and upload. A bad seed (--offset 400) costs 43 renders for the first song, then the learned offset fixes the rest (8 songs: 9.2 s with 1 job, 6.8 s with 2, 6.0 s with 4). With a good seed 1, 2 and 4 jobs take about the same, so the default is 2 jobs (permits are shared).

Failed hypothesis / finding: parallel rbhost processes did not work at first: the engine answered 'K' (out of memory, code 23). The DLL's two ring buffers are named kernel objects of the whole Windows session ("ReBirth DLL->Panel" / "Panel->DLL" with _SEMA/_EVENT, docs/export_state.md 2.7), so processes share one ring. Fix in rbhost.c: --private-port overwrites the first 8 characters ("ReBirth ") of the two name literals in the loaded image with the process id in hex (found by scanning the sections, exactly one occurrence each, checked) and rbhost opens the same names. Only object names change; the default (without the option) is unchanged and its hashes still equal the parallel ones (303_slide N = 9120, db11fa67... in the serial run, same N in the parallel fit).

AC #3 re-run (8 songs: 303_slide [303 2.0], 303_vintage_square [Vintage], 808_bass_knobs_hi, 909_clap, delay_straight_fb127 [effect], tempo_66_667 [odd tempo], comp_ch_a127_t0, pcf_on_909 [task-012's outlier]; task-012's fits for them were 9104, 9104, 9056, 9152, 9136, 9072, 9072, 9264): N is not reproducible between sessions (it follows the timeline, export_state.md 11.6), so equal N cannot be asked; the verdict is asked: each export has exactly one matching N, byte for byte (whole-file sha256), and it equals the N the old serial run.py --idle-sweep --match finds on the same WAV. Done for 3 app runs x 8 songs (T = 10 with the new polls: N 8160..8224; T = 2: 2640..2704; T = 0: 1296..1376; each fit unique among 31 candidates with --all) and for the final default run (T = 2): N = 2688, 2672, 2704, 2656, 2640, 2672, 2704, 2640, unique among 31 candidates, and the serial run.py sweep over 40 values found the identical N for all 8; the 16 exports at T = 10 and T = 2 also agree between fit.py and the serial sweep. N - N_t = 38..65 (T = 10), 24..79 (2), 40..88 (0), 29..77 (default): inside task-012's 3..82 except 88 at T = 0; pcf_on_909 had no late export start this time (38, 62, 54, 37). Silent songs (all-zero WAVs match every N) are not in the sample; fit.py reports all N for them with --all.

Unconfirmed / not done: fits of several songs of one app session together (a fresh process per song gives each its own N); several candidates in one rbhost process (a process is 0.2 s a render; a second EngineOpen was not tried); the open phase (2.7 s: Ctrl+O, typing, load) and the File-menu keys (1.0 s) were not shortened further. Whether a shorter T than 2 s is safe on a slower machine is not known: T = 0 worked for the 8 songs only.

Incidents: one RCS name lookup failure (gnome.lan) in 100+ calls killed a fit run: fit.py now retries network errors (status 0) 3 times. A foreign Rebirth.exe (PID 14500, session RDP-Tcp#0, not started by me) ran for a few minutes while no permits were held; rb_export.py and run.py refused to start as designed; left alone, it ended by itself.

Tests: python3 unittest tools (OK) and tools/rbhost (20 tests, OK, fit_test.py new); the runs above; --test-fail (permits released, app stopped); run.py --match-hash on 303_slide (3 hashes differ, the 4th is downloaded and identical).

Review round 1 (send back), all ten items addressed, amended into the one commit (no machine run; the Python tests only):
1. Commit subject shortened to under 72 characters.
2. fit.py: an all-zero app export (checked from <app-dir>/<stem>.wav) is fitted with one render at the seed, reported as "all N" (all_n in fit.json), rc 0, and kept out of the offset median. Unit test added.
3. Default mode reports the FIRST match; uniqueness only with --all. Stated in fit.py --help and docs/export_state.md 11.6 (I did not add the one-N-each-side check: it would double the renders for little).
4. AC #3 reinterpretation, explicit: N is not reproducible between app sessions (it follows the timeline), so equal N to task-012's cannot be the test. What was checked: every export has exactly one matching N among 31 candidates (fit.py --all), and the old serial run.py --idle-sweep --match finds the same N on the same WAVs (16 exports at T = 10 and 2, and the 8 of the final default run; not done for the T = 0 set).
5. MIDI-box vs main-window rule: no cheap second property from rcs (screenshot is the only window query), so the limit and the failure mode are documented in docs/test_machine.md section 8 and at the constant.
6. docs/export_state.md 11.6: T = 2 repeatability (section 11.1 "identical x3") marked [G, not measured]; the corpus sentence reworded (corpus references are rbhost at a fixed N); the T = 0 claim marked [G].
7. docs/export_state.md 2.7 has the two literal addresses (VA 0x10152D18 and 0x10152EE8, .data1, one occurrence each, re-checked in original/ReBirth.dll) and the rbhost.c comment cites them.
8. fit_test.py: tests for the median offset, the first-match stop (and --all), the learned offset between songs, the all-N case, silent-WAV detection, exec not retried, and the permit release plus fit.json after an unexpected error (fake permits and client).
9. fit.py: exec of an rbhost run is never retried (only proc, output, hash are); main catches any Exception, so fit.json is written and the permits are released.
10. docs/test_machine.md line wrapped.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
App export 25.4 s -> 9.6 s per song (7.8 s at T = 0); N fit 7-8 s (serial sweep to the match; 36 s for the 49-value sweep) -> 0.2-0.6 s per song. rb_export.py polls the MIDI box, the main window and the file instead of fixed sleeps, default --wait-after-open 2 (was 10), one upload batch, hash check of the download. New tools/rbhost/fit.py seeds N at N_t + offset from the sidecar timeline, runs rbhost processes in parallel (new rbhost --private-port, because the DLL's ring buffers are session-wide named objects), compares hashes on the machine; reports the first match (unique only with --all); all-zero exports are reported as all N. run.py: --match-hash, one tasklist/mod upload per song. AC #3 as re-interpreted: N is not reproducible across app sessions (it follows the timeline), so equal N to task-012 cannot be asked. Checked on 8 songs (303 2.0, Vintage, 808, 909, delay, odd tempo, compressor, pcf_on_909) in 4 app runs: exactly one matching N per export with --all among 31 candidates, and the old serial run.py sweep finds the same N on the same WAVs (T = 10, 2 and the final default run; not T = 0). T = 2 repeatability and T = 0 are [G]. Docs: export_state.md 2.7 and 11.6, test_machine.md, rbhost README. Branch worktree-agent-a4af81832aa319040, one commit, review round 1 amended. Tests: python unittest tools and tools/rbhost pass (fit_test.py new); tool runs in the notes.
<!-- SECTION:FINAL_SUMMARY:END -->
