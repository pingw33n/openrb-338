---
id: TASK-068
title: 'Rename the project to OpenRB-338: binary orb, ORB_ environment variables'
status: To Do
assignee: []
created_date: '2026-10-27 11:27'
updated_date: '2026-10-28 11:48'
labels:
  - cli
  - tools
  - docs
  - tests
milestone: m-4
dependencies: []
references:
  - AGENTS.md
priority: medium
type: chore
ordinal: 94000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requested by the user on 2026-10-27. Unassigned, not in a milestone.

Rename the project from rb to OpenRB-338 (the user's name, 2026-10-27; first written as "OpenRB"), with the binary `orb`, and every environment variable to the `ORB_` prefix.

Scope:
- Binary: crates/rb-cli's [[bin]] name "rb" becomes "orb"; every user-facing string that names the command (help, errors such as "run `rb recrypt ...`" and "`rb standard-mod --rebirth-dll ...`", the default Standard.rbm next to the binary, `rb play`'s messages) says `orb`.
- Crates: rb-cli, rb-engine, rb-player (and rb-ui when task-044 adds it) become openrb-* or orb-* (choose one, say why; directories, package names, `use` paths). The rbm and rbs crates keep their names: RBM and RBS are ReBirth's file formats, not the project's name. The same rule everywhere: "rb" meaning this project becomes OpenRB-338 (prose; the full name, as the original is ReBirth RB-338) / orb (command, crate prefix if chosen); RB-338, ReBirth, .rbs, .rbm, RB20FUL.DAT and the like are the original's names and stay.
- Environment variables: RB_RB20FUL, RB_MOD_DIR, RB_RBM_DIR (a deprecated alias: drop it or keep it as ORB_RBM_DIR; say which), RB_MOD_ORIG_DIR, RB_STANDARD_MOD, RB_THIRDPARTY_DIR, RB_CD_ISO (task-065), RB_CD_REBIRTH_EXE (task-067), and any other RB_ variable in crates/, tools/, tests, docs/ and the backlog docs become ORB_* (ORB_RB20FUL, ORB_MOD_DIR, ...). Decide whether the old names stay as deprecated aliases for a while (suggested: no, the project has no outside users); say which.
- Python tools (tools/, tools/rbhost/) that invoke the binary or read the variables; tools/rbhost and rbcorpus.py keep their names unless they name the project (say what you renamed).
- Docs and agent instructions: AGENTS.md (title "the rb repository", commands, crate list), backlog doc-001 (the agent brief, via the backlog CLI), .claude/agents/*.md, docs/*.md, tests/refs/MANIFEST.md (generated: change the generator tools/rbcorpus.py and regenerate), the Cargo workspace, README if any. Open backlog tasks (To Do) whose descriptions name `rb ...` commands or RB_ variables get updated through the backlog CLI; Done tasks, their notes and the git history stay as written (they are history).
- On the test machine: rb's working folder C:\re\rb and its paths in tools and docs may stay (they are the machine's layout, not the product name) or move to C:\re\orb; say which, and do not touch C:\ReBirth RB-338.
- Out of scope and for the user: renaming the GitHub repository (github.com/pingw33n/rb) and the local checkout folder (/Users/pingw33n/dev/rb). List what would follow from those (the git remote, the Claude memory folder name, absolute paths in docs) in the final summary, without doing it.

Run at a quiet point: the rename touches almost every file, so no other task branch should be open (rebase conflicts); check `git worktree list` and the queue first. After the rename: cargo build/test --release (with the mod env vars under their new names), clippy, fmt, the Python tests, `orb compare --manifest ...` 164/164 (175/175), and `git grep -nwE 'rb|RB_[A-Z]'` reviewed so every remaining hit is a deliberate keep (list the kept classes).
Crate prefix: "openrb-338-" is awkward; "orb-" (the binary's name) or "openrb-" are fine; say which. The window title of the UI (m-3) and the app's About text say "OpenRB-338".
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The binary is orb, the crates follow the chosen prefix, and every user-facing message names orb; rbm/rbs and the original's names are unchanged
- [ ] #2 Every environment variable uses the ORB_ prefix in code, tests, tools and docs; whether old names remain as aliases is stated
- [ ] #3 AGENTS.md, the agent brief, .claude/agents, docs and open tasks name the project OpenRB-338 and the command orb; Done tasks and history are untouched; a git grep review lists only deliberate keeps
- [ ] #4 cargo test --release, clippy, fmt, the Python tests and orb compare (164/164, 175/175) pass; the final summary lists what renaming the GitHub repo and the local folder would change
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
task-104 already named the desktop UI OpenRB-338 (binary, window title, app name, app_id); the rest of the rename is still here.
<!-- SECTION:NOTES:END -->
