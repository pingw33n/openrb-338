---
id: TASK-066
title: 'Remove Rebirth.exe, ReBirth.dll and the IDA databases from git history'
status: To Do
assignee: []
created_date: '2026-10-27 10:07'
updated_date: '2026-10-27 11:16'
labels:
  - tools
  - tests
  - docs
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
- [ ] #1 No commit in the rewritten history (local and origin) contains original/Rebirth.exe, original/ReBirth.dll or anything under ida/; a fresh clone shows none
- [ ] #2 The files stay on disk, are ignored by git, and original/README.md says what to put there; a backup of the pre-rewrite repository exists and its location is recorded
- [ ] #3 Tests and tools work with the files present and skip with a clear message when absent; AGENTS.md and the agent brief describe original/ and ida/ as local; cargo test --release passes
- [ ] #4 text.s and any other disassembly or binary derived from the original (except the approved embedded key window) are gone from the history
<!-- AC:END -->
