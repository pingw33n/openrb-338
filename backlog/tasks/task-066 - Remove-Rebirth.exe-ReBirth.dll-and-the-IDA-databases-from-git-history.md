---
id: TASK-066
title: 'Remove Rebirth.exe, ReBirth.dll and the IDA databases from git history'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-27 10:07'
updated_date: '2026-10-29 07:51'
labels:
  - tools
  - tests
  - docs
milestone: m-4
dependencies: []
references:
  - AGENTS.md
priority: medium
type: chore
ordinal: 92000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requested by the user on 2026-10-27. Unassigned, not in a milestone.

The repository tracks Propellerhead's program and the IDA databases made from it: original/Rebirth.exe, original/ReBirth.dll, ida/Rebirth.exe.i64 and ida/ReBirth.dll.i64 (all four added in the first commit, 9b18909; the .i64 files changed since). Remove them from the whole git history, so no commit on main or on github.com/pingw33n/rb (private) holds them, while the files stay on disk where the tools and the user's IDA expect them.

This rewrites every commit hash and needs a force-push, so it must be run by or with the user, at a quiet point:
1. Before: no agent running, every worktree merged or removed (git worktree list; the leftover locked worktrees of tasks 038 and 058 and the unmerged branch worktree-ui-poc, commit 1359e39, need the user's decision: rebase it onto the rewritten main or drop it); the user's IDA GUI closed (it saves to ida/Rebirth.exe.i64). Back up the whole repository first (e.g. `git clone --mirror` to a folder outside it, plus a copy of original/ and ida/), and say where.
2. Rewrite with git filter-repo (`--invert-paths --path original/Rebirth.exe --path original/ReBirth.dll --path ida/`; check what else under original/ and ida/ exists in history with `git log --all --name-only -- original ida`). Keep the files in the working tree: copy them out before, back after.
3. Add original/*.exe, original/*.dll and ida/*.i64 to .gitignore, and a short original/README.md (committed) saying what goes there (the user's own installation's Rebirth.exe and ReBirth.dll with their SHA-256: ReBirth.dll 4c3a6441a2cd50f797d99b1c5b6cf0aaf198c7ad8ca2648196b530c2e3e58b12; Rebirth.exe as in docs/test_machine.md) and that rb never needs them except for the tests and the reverse engineering.
4. The tests and tools that read original/ (about 20 files: crates/*/tests, tools, .claude/agents, backlog/docs/doc-001) keep working with the files present and skip with a clear message when they are absent (check how each fails today without them: the golden test, corpus tests, rb-cli tests). Update AGENTS.md (which says the original program is in original/ and the IDA databases in ida/) and the agent brief to say they are local, untracked.
5. Force-push main (the user confirms first), then check on GitHub that no blob of the four remains (e.g. a fresh clone and `git log --all -- original ida` empty); run backlog doctor, cargo test --release. Note that GitHub may keep old commits reachable by hash for a while (cached views, forks, pull requests); say what the user can do (GitHub support's "remove cached views", or recreating the repository).

Also remove text.s (added by mistake in commit 3389512, a 3.4 MB llvm-objdump disassembly of original/ReBirth.dll; untracked and git-ignored on 2026-10-27 but still in history): add `--path text.s` to the filter-repo call, and check the history for any other disassembly listing or binary derived from the original (e.g. `git log --all --diff-filter=A --name-only | grep -iE '\.(s|asm|i64|idb|dll|exe|bin)$'`; crates/rbm/src/rb20ful_window.bin is the user-approved embedded key window and stays).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 No commit in the rewritten history (local and origin) contains original/Rebirth.exe, original/ReBirth.dll or anything under ida/; a fresh clone shows none
- [x] #2 The files stay on disk, are ignored by git, and original/README.md says what to put there; a backup of the pre-rewrite repository exists and its location is recorded
- [x] #3 Tests and tools work with the files present and skip with a clear message when absent; AGENTS.md and the agent brief describe original/ and ida/ as local; cargo test --release passes
- [x] #4 text.s and any other disassembly or binary derived from the original (except the approved embedded key window) are gone from the history
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Rebased on main fc03593. 2. Untrack the four files (cached removal); .gitignore original/* and ida/* except READMEs; original/README.md and ida/README.md. 3. Audit readers of original/ and ida/ with grep: crates already use rb-testkit RB_CD_ISO; make tools/gen_static_tables.py fail with a clear message when the DLL is absent. 4. AGENTS.md, .claude/agents, doc-001 (CLI): original/ and ida/ are local and untracked. 5. Run cargo test --release with and without RB_CD_ISO in the worktree (files absent); fmt, clippy, shellcheck, actionlint. 6. History search of added paths for derived files; filter-repo path list in the notes. 7. Check AC 2 (disk, ignore, README part; the backup is the supervisor's phase 2) and AC 3; leave AC 1 and 4; Done.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
User, 2026-10-29: go ahead with task-066. Decisions:
- Drop the user's ui-poc branch and worktree (worktree-ui-poc, 1359e39). The supervisor removes it before the rewrite.
- Keep crates/rb-engine/tests/fixtures/tables/*.bin. They are run-time math tables dumped by rbhost, like the reference renders in tests/refs, which also stay.

Plan (supervisor):
- Phase 1, @agent-worker, one reviewed commit: original/README.md; .gitignore for original/* and ida/* (except the README); git rm --cached of the four tracked files; AGENTS.md, the agent brief and .claude/agents describe original/ and ida/ as local and untracked; tests and tools skip with a clear message without them; a history search for other derived binaries. That commit sets the task Done with AC #2 and #3 checked.
- Phase 2, the supervisor with the user: back up (git clone --mirror plus a copy of original/ and ida/); the user closes the IDA GUI (it has original/Rebirth.exe open); git filter-repo; verify; force-push only after the user confirms. AC #1 and #4 are then checked in a backlog commit after the push is verified.

Branch (phase 1): worktree-agent-a419964b67de26e6e

Phase 1 (worker), branch worktree-agent-a419964b67de26e6e, on main fc03593.

Done:
- Untracked the four files (git rm --cached). In this worktree I then deleted them from disk, so original/ and ida/ were absent for every test below. The main checkout's original/ and ida/ were only read (shasum): ReBirth.dll 4c3a6441...8b12 (2,472,960 bytes), Rebirth.exe 6a651e81...f876 (1,594,880 bytes; the hash is also in docs/rbm.md 13, docs/test_machine.md has no full hash of it).
- .gitignore: /original/* (except README.md), /ida/* (except README.md), plus *.i64 and *.idb anywhere; merged with the IDA working-file entries. text.s and *.asm entries were already there.
- original/README.md (what goes there, both SHA-256, sizes, rb needs them only for the RE and the tests, RB_CD_ISO gives the tests the same files, the patched exe differs from the CD's e69fef41...7004) and ida/README.md.
- AGENTS.md (opening paragraph, standing rule), .claude/agents/{chore,complex,reviewer,worker}.md and backlog doc-001 (Layout table, IDA step 1: copy the database from the main checkout's ida/, reading only) say original/ and ida/ are local and untracked.
- Audit of readers (git grep of original/, ida/, ReBirth.dll, Rebirth.exe, .i64 in crates, tools, scripts, .github, .claude, docs): the Rust tests already take the DLL/exe from RB_CD_ISO (task-121) and read nothing under original/ or ida/; the only reader left was tools/gen_static_tables.py (default --dll original/ReBirth.dll), which now exits 1 with a message saying where to put the file or to pass --dll. Mentions in docs/*.md and doc comments are provenance text, not reads.

Tests (worktree without original/ and ida/):
- cargo test --release -q --no-fail-fast, RB_CD_ISO unset: 586 passed, 0 failed, 3 ignored. Skip lines: "skipped: RB_CD_ISO (the ReBirth 2.01 CD image) is not set" (about 40 times), "skipped: the recrypted shipped mods need RB_CD_ISO" (3), "skipped: RB_THIRDPARTY_SONGS_DIR ... is not set", "skipped: RB_THIRDPARTY_MODS_DIR ... is not set".
- RB_CD_ISO=/Users/pingw33n/Downloads/Rebirth 2.01 (2001 version).iso: 586 passed, 0 failed, 3 ignored; remaining skips only the two RB_THIRDPARTY_* ones (and "the file system folds case").
- cargo +nightly fmt --all --check ok; cargo clippy --all-targets -- -D warnings ok; shellcheck scripts/*.sh ok; actionlint ok; tools/gen_static_tables.py without the DLL: exit 1 with the message.

Not confirmed / left to the supervisor:
- AC #2 backup (git clone --mirror plus a copy of original/ and ida/, location recorded) is phase 2, the supervisor's. This commit satisfies the disk/ignore/README part. WARNING for the merge: this commit deletes the four tracked files, so merging it (fast-forward or squash) into the main checkout REMOVES them from its working tree, where the user's IDA GUI has original/Rebirth.exe open. Before merging, copy original/ and ida/ out of the main checkout (cp -p) and put them back after the merge, then check shasum: ReBirth.dll 4c3a6441...8b12, Rebirth.exe 6a651e81...f876.
- AC #1 and #4 are unchecked: phase 2.

History search (git log --all --diff-filter=A --name-only, 1103 distinct added paths; scan of all blobs > 50 kB for MZ / IDA headers; blob size list):
Certainly derived (remove):
- original/ReBirth.dll (9b18909, blob a83bad73, 2.4 MB, PE)
- original/Rebirth.exe (9b18909, blob d0b29541, 1.5 MB, PE)
- ida/ReBirth.dll.i64 (9b18909, blob 50ad82d1, 5.3 MB, IDA2)
- ida/Rebirth.exe.i64 (9b18909, blob 1563dbb5, 14.8 MB, IDA2)
- text.s (3389512, blob a1b854c2, 3.4 MB llvm-objdump listing)
No other PE or IDA blob exists in history; the scan found no other MZ/IDA2 file, no .s/.asm/.idb/.id0/.dll/.exe added elsewhere.
Not derived binaries, listed for the user's judgment (the approved ones are kept):
- docs/test_machine/*.png (7007079, f031894): 10 screenshots of the original program's UI on the test machine.
- docs/ui_spec/panel.png, panel_controls.png, transport_song_mode.png (f00b373): screenshots of the original's panel, cropped (panel_controls has rectangles drawn over one). docs/ui_spec.md section 14 says so.
- tests/songs/shipped/*.rbs (7007079, 18 songs) and tests/refs/*: songs shipped with the installation (user decision task-006) and reference renders (approved).
- Not derived: assets/ui/** (generated by crates/rb-ui-art, own art, assets/ui/README.md "License and origin"), docs/ui_app/*.png (screenshots of rb-ui), assets/app_icon.ico (90c312e, generated from our svg; not verified byte by byte), tests/refs/thirdparty_mods.json (hash manifest, 2 MB).
- Approved, keep: crates/rbm/src/rb20ful_window.bin (c229e9c), assets/Standard.rbm (14997b0), tests/refs/**, crates/rb-engine/tests/fixtures/tables/*.bin (b8a3234).
Exact filter-repo call for the supervisor:
  git filter-repo --invert-paths --path original/Rebirth.exe --path original/ReBirth.dll --path ida/Rebirth.exe.i64 --path ida/ReBirth.dll.i64 --path text.s
Use the four file paths, not the directories original/ and ida/: this commit adds original/README.md and ida/README.md, which the directories would delete from the new history. git log --all --name-only -- original ida text.s shows exactly these five paths. The screenshots above are not in the list; add them (--path docs/test_machine/ --path docs/ui_spec/ plus the markdown links in docs/test_machine.md and docs/ui_spec.md) only if the user decides so.

Review round 1 (send back, 3 fixes): (1) backup recorded: the supervisor made it on 2026-10-29 09:3x at ~/rb-backup-2026-10-29/: rb.git (git clone --mirror of the local repo at fc03593, all branches including worktree-ui-poc), original/ and ida/ copied with cp -Rp, original.sha256 and ida.sha256 listing every file; the IDA GUI is closed. AC #2 stays checked. (2) removed the stale original/** -text rule and its comment from .gitattributes. (3) original/README.md no longer says the tests compare against the files.

Phase 2, supervisor, 2026-10-29:
- Backup: ~/rb-backup-2026-10-29/ holds rb.git, a git clone --mirror at fc03593 with all branches including worktree-ui-poc and the old history, plus original/ and ida/ copied with cp -Rp, and original.sha256 / ida.sha256.
- Phase 1 was merged as bdb65be (old hash). original/ and ida/ were copied out and back, and every file passed shasum -c against the backup lists.
- The ui-poc worktree and branch were removed (user's decision), along with the phase-1 agent worktree.
- git filter-repo 2.47.0 (brew) ran with --invert-paths --path original/Rebirth.exe --path original/ReBirth.dll --path ida/Rebirth.exe.i64 --path ida/ReBirth.dll.i64 --path text.s, plus a commit callback that adds LICENSE (blob f288702d, the verbatim GPLv3) to the root commit (the user's request of 2026-10-29).
- Result: 222 commits, and the new root 59aaab1 contains LICENSE.
  - git log --all on the five paths is empty, and the four old blobs (1563dbb5, 50ad82d1, a83bad73, d0b29541) are absent.
  - The tree of HEAD^ differs from the backup's fc03593 only by the four removed files.
  - .git went from 64 MB to 23 MB.
  - cargo test --release passed 586, failed 0, ignored 3; backlog doctor was clean.
- Force-pushed with --force-with-lease=main:9866fc1 after the user confirmed (9866fc1...35a40e7). A fresh clone of github.com/pingw33n/rb shows 222 commits, no hits for the five paths, none of the four blobs, and LICENSE in the root; ls-remote shows only main.
- Left over: GitHub may keep the old commits reachable by hash for a while (cached views). To purge them, ask GitHub Support to remove cached views for the repository, or recreate the repository.
- docs/ and backlog/ text still cite some old commit hashes; filter-repo rewrote the ones in commit messages, not those in file contents.

2026-10-29, user request (branch worktree-agent-a5cf9f52f3f3e46c0): remove the original/ and ida/ folders and every mention of them from the repository; the user's local folders stay on disk, task files unchanged. Removed: original/README.md, ida/README.md, their .gitignore rules; mentions rewritten in AGENTS.md, .claude/agents, doc-001, docs, assets/README.md, code comments; tools/gen_static_tables.py now requires --dll or RB_REBIRTH_DLL. git grep for original/ and ida/ outside backlog/tasks is empty. fmt, clippy, shellcheck, actionlint clean; cargo test --release 586 pass 0 fail with and without RB_CD_ISO.

Review round 1 (2026-10-29): restored the facts lost with original/README.md (installed ReBirth.dll 2,472,960 bytes and the full SHA-256 of the installed Rebirth.exe) in docs/rbm.md 13.1 and assets/README.md; the IDA databases path is given by the supervisor in each prompt (doc-001, agent-complex.md); subject area changed to docs:; rebased onto 9ad4543.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Phase 1 done on branch worktree-agent-a419964b67de26e6e: the four files are untracked, original/ and ida/ are git-ignored (READMEs kept), original/README.md and ida/README.md say what goes there, AGENTS.md, .claude/agents and doc-001 describe them as local and untracked, tools/gen_static_tables.py fails with a clear message without the DLL. Tests in a worktree without the files: 586 passed, 0 failed with and without RB_CD_ISO (skip lines in the notes). AC 1 and 4 and the backup part of AC 2 are phase 2 (supervisor); the history search and the exact filter-repo path list are in the notes.
<!-- SECTION:FINAL_SUMMARY:END -->
