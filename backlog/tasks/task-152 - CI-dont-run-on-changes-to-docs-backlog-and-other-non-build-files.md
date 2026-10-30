---
id: TASK-152
title: 'CI: don''t run on changes to docs, backlog and other non-build files'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-30 07:52'
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
- [ ] #1 ci.yml's push and pull_request triggers ignore backlog/**, docs/**, .claude/**, **.md, LICENSE and assets/readme/**; the README pictures live in assets/readme/ with the links updated; no test reads an ignored path (checked); the rule is documented in ci.yml and docs/building.md; actionlint passes
<!-- AC:END -->
