---
id: TASK-152
title: 'CI: don''t run on changes to docs, backlog and other non-build files'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-30 07:52'
updated_date: '2026-10-30 08:29'
labels:
  - tools
milestone: m-5
dependencies: []
references:
  - .github/workflows/ci.yml
  - README.md
  - assets/README.md
  - docs/building.md
priority: medium
type: chore
ordinal: 170000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked on 2026-10-30 that ci.yml stop running automatically for pushes and pull requests that change only files which no build or test reads. Today every push to main runs fmt, clippy and the tests on three OSes, including the many backlog-only commits.

What:
- In .github/workflows/ci.yml, add `paths-ignore` to both the `push` and the `pull_request` triggers, with these patterns (the user's choice):
  - `backlog/**`
  - `docs/**`
  - `.claude/**`
  - `**.md` (every Markdown file: README.md, AGENTS.md, the crate and fixture READMEs, tests/refs/MANIFEST.md)
  - `LICENSE`
  - `assets/readme/**`
- Move the README's pictures into assets/readme/ so that one rule covers them: assets/logo.gif becomes assets/readme/logo.gif, and assets/screenshot.png becomes assets/readme/screenshot.png. Update the links in README.md and the list in assets/README.md. Use git mv.
- Keep running CI on everything else, in particular:
  - scripts/ (crates/orb-ui/src/version.rs runs scripts/version.sh in a test);
  - tools/ (the cargo tests run tools/*.py);
  - tests/, crates/, assets/ui/, assets/Standard.rbm, assets/app_icon.ico;
  - Cargo.*, rust-toolchain.toml, rustfmt.toml, clippy.toml, and .github/workflows/.
  Verify the claim that no test reads docs/ or a .md file: git grep for include_str!, fs::read and open of any .md, or anything under docs/, in crates/ and tools/. If something does, leave that path out of the ignore list and say so.
- GitHub's rule: a push is skipped only if every changed file matches the ignore list, so a mixed commit still runs CI. Document this, and the list, in a comment in ci.yml and in docs/building.md (the CI section).
- release.yml (tag pushes) is unaffected; keep it so.
- actionlint must pass. The real check is the next pushes after the merge: a backlog-only push runs no ci job, and a code push runs it. The supervisor checks both and records them in this task's notes after the merge.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ci.yml's push and pull_request triggers ignore backlog/**, docs/**, .claude/**, **.md, LICENSE and assets/readme/**; the README pictures live in assets/readme/ with the links updated; no test reads an ignored path (checked); the rule is documented in ci.yml and docs/building.md; actionlint passes
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. git mv the two README pictures to assets/readme/, fix links in README.md and assets/README.md. 2. Add paths-ignore (6 patterns) to push and pull_request in ci.yml with a comment. 3. Document in docs/building.md CI section. 4. Verify by git grep that no test/build.rs reads an ignored path. 5. actionlint.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-aac1c1f4dc250842e (rebased on main 50d25ff)

Grep findings (git grep in crates/, tools/, tests/, scripts/): every docs/ and .md mention is a comment, doc comment or message string; no include_str!/include_bytes! of .md, docs, LICENSE or the pictures; no fs::read/open of them; build.rs files only watch crates/orb-ui, assets/ui, .git and the .ico. tests/refs/MANIFEST.md (made by tools/rbcorpus.py manifest) is checked by no test (corpus_golden only mentions rbcorpus.py check-flac in a message; it reads manifest.json, which is not ignored). LICENSE is read only by the release package scripts (package-appimage.sh, package-dmg.sh, appimage-smoke.sh), not by CI tests. Other .md under crates/ and tests/ (fixtures/tables/README.md, tests/songs/diag/README.md) are read by no test. So the whole list is safe, nothing left out. YAML anchors not used (the two lists are duplicated). actionlint passes. No Rust changed.

Supervisor, post-merge check, 2026-10-30:
- The merge push of b271ff7 (it changes ci.yml, so a code push) ran ci: run 41023332888, success.
- The next push holds only backlog commits (task-037/144 backlog, task-153, and this note); it is checked for no ci run.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
ci.yml push and pull_request have paths-ignore for backlog/**, docs/**, .claude/**, **.md, LICENSE, assets/readme/**; pictures moved with git mv to assets/readme/, links in README.md and assets/README.md updated; rule documented in ci.yml and docs/building.md; grep check recorded in notes; actionlint ok. No Rust changed, so no cargo run. Branch worktree-agent-aac1c1f4dc250842e. The real check (backlog-only push runs no ci job, code push does) is for the supervisor after merge.
<!-- SECTION:FINAL_SUMMARY:END -->
