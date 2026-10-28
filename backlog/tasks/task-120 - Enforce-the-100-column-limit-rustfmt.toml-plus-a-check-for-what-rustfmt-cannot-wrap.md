---
id: TASK-120
title: Enforce the 100-column limit with nightly rustfmt
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 13:27'
updated_date: '2026-10-28 13:50'
labels:
  - tools
  - tests
milestone: m-4
dependencies: []
references:
  - rustfmt.toml
  - AGENTS.md
priority: medium
type: chore
ordinal: 140000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) that the 100-column line limit be a rule in rustfmt.toml, enforced by nightly rustfmt; it is already installed here. Reviews keep catching lines over 100 columns, for example a 114-column raw string in crates/rb-ui-art/tests/logo.rs (task-116).

Facts:
- The workspace builds with the pinned stable toolchain (rust-toolchain.toml, 1.99.1), and that stays.
- Only formatting moves to nightly rustfmt. It is installed here as the toolchain nightly-aarch64-apple-darwin, rustfmt 1.11.0-nightly (b6eeb93c99 2026-10-18), and runs as `cargo +nightly fmt`.
- Stable rustfmt ignores the unstable options and prints "can't set ..., unstable features are only available in nightly channel".
- rustfmt's max_width already defaults to 100. Strings and comments are what it doesn't wrap or report on stable.
- `git ls-files '*.rs' | xargs awk 'length > 100'` finds 31 lines today, most of them comments and strings, for example in crates/rb-ui-art/src/font.rs.

What:
1. rustfmt.toml:
   - max_width = 100 (explicit);
   - error_on_line_overflow = true;
   - wrap_comments = true and comment_width = 100, so nightly rewraps comments itself;
   - any other unstable option that helps without churn; say which, and why format_strings is or isn't set. It is known to be buggy and doesn't touch raw strings, so prefer splitting long strings by hand.
   Add a comment saying the file needs nightly rustfmt.
2. Run `cargo +nightly fmt --all` once and commit the result in this task's one commit. Review the diff: comment rewraps should not reflow tables, ASCII diagrams, doc-test code or URLs badly. Protect any such blocks (for example with `// rustfmt::skip` items, or by keeping them in doc lists rustfmt won't wrap), and say what was protected.
3. Fix by hand what nightly still reports, such as long string literals: use concat! or \ continuations. `cargo +nightly fmt --all --check` must pass with no overflow error.
4. Pin the nightly for CI: name a dated nightly (the installed one's date, nightly-2026-10-18, or a nearby one that has rustfmt) in docs/building.md and AGENTS.md. Task-108's ci.yml must run `cargo +nightly-<date> fmt --all --check` (rustup toolchain install nightly-<date> --component rustfmt). If ci.yml does not exist yet, record this in task-108's notes through the backlog CLI so its agent adds it.
5. AGENTS.md:
   - Build and test: Format becomes `cargo +nightly fmt --all`, check `--check`.
   - Standing rules: lines at most 100 columns, enforced by nightly rustfmt.
   - Say that plain `cargo fmt` (stable) still formats but does not enforce the limit and warns about the unstable options.
   Also update the agent brief if it names cargo fmt: backlog doc-001, through the backlog CLI.
- Markdown docs (docs/*.md) are not covered; their long table rows stay as they are.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 rustfmt.toml sets max_width = 100, error_on_line_overflow, wrap_comments and comment_width = 100 (commented as nightly-only); cargo +nightly fmt --all --check passes with no overflow error
- [x] #2 The existing over-long lines are fixed (comments rewrapped by nightly, strings split by hand); no tracked .rs line exceeds 100 columns (awk check in the notes)
- [x] #3 AGENTS.md, docs/building.md (or the agent brief) name the pinned nightly and the fmt commands; CI runs the nightly fmt check (or task-108's notes say to); cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. rustfmt.toml: max_width, error_on_line_overflow, wrap_comments, comment_width (nightly-only, commented). 2. cargo +nightly fmt --all, review diff for mangled tables/diagrams/URLs, protect with #[rustfmt::skip] or restructure. 3. Hand-split remaining long strings. 4. Pin nightly-2026-10-18 in docs/building.md, AGENTS.md; ci.yml or task-108 notes. 5. AGENTS.md + doc-001 fmt commands. 6. fmt check, clippy, tests, corpus.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-ab7bb2dc16f03cd98

Ran cargo +nightly fmt --all: 22 files reflowed (comment rewraps in //! and /// lists, and macro invocations Delay/Pcf/Distortion/Compressor table split one arg per line). Reviewed the diff: rewraps only reflow prose lists; no table, diagram, URL or doc-test was touched, so nothing needed rustfmt::skip. Nightly still left 22 lines over 100 that it does not report (long strings in format!/write!, json! macro, trailing comments, a Python raw string): fixed by hand with backslash continuations (font.rs, svg.rs, layout.rs, aiff.rs, main.rs, thirdparty.rs), a Python line split (thirdparty.rs), json! lines split (rbs/tests/corpus.rs), comment moved (mods.rs, tb303.rs). concat! was tried for raw format strings and fails (implicit captures do not pass through concat!), so continuation strings were used. format_strings not set: buggy, and does not touch raw strings. No other unstable option added (no churn wanted). Check: awk 'length > 100' over all tracked .rs files gives 0 lines; cargo +nightly fmt --all --check exits 0. Diff stat of crates and rustfmt.toml: 28 files changed, 162 insertions, 121 deletions. clippy clean; cargo test --release 540 pass, 0 fail, 3 ignored; corpus compare 175 of 175 equal (manifest now has 175 entries); cargo doc --no-deps shows only unresolved-link warnings for [C] in existing doc text (pcf.rs). CI: no ci.yml or docs/building.md exist yet, so task-108's notes got the nightly-2026-10-18 fmt step; AGENTS.md names the pinned nightly and commands. Agent brief doc-001 does not mention cargo fmt, unchanged.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
rustfmt.toml now sets max_width = 100, error_on_line_overflow, wrap_comments and comment_width = 100 (nightly-only, commented). The workspace was reformatted once with nightly rustfmt (nightly-2026-10-18); the 22 lines it cannot wrap were fixed by hand, so no tracked .rs line exceeds 100 columns. AGENTS.md has the fmt commands, the pinned nightly and the line rule; task-108's notes carry the CI fmt step. Checks: fmt --check passes, clippy clean, cargo test --release 540 pass (3 ignored), corpus 175 of 175 equal. No behavior change. Branch worktree-agent-ab7bb2dc16f03cd98, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->
