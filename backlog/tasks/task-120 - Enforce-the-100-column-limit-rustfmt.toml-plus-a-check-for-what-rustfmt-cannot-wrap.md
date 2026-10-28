---
id: TASK-120
title: >-
  Enforce the 100-column limit: rustfmt.toml plus a check for what rustfmt
  cannot wrap
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 13:27'
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
Why: the user asked (2026-10-28) that the 100-column line limit be a rule in rustfmt.toml. Reviews keep catching lines over 100 columns, for example a 114-column raw string in crates/rb-ui-art/tests/logo.rs in task-116.

Facts:
- rustfmt.toml has only edition = "2024".
- rustfmt's max_width defaults to 100, so rustfmt already wraps code at 100.
- rustfmt does not break string literals or comments, and rustfmt also gives up silently on some long expressions.
- The options that would report or wrap those (error_on_line_overflow, error_on_unformatted, wrap_comments, comment_width) are unstable. On the pinned stable toolchain (rust-toolchain.toml, 1.99.1) rustfmt prints "can't set error_on_line_overflow = true, unstable features are only available in nightly channel" and ignores them.
- `git ls-files '*.rs' | xargs awk 'length > 100'` finds 31 lines today, most of them comments and strings, for example in crates/rb-ui-art/src/font.rs.

What:
1. rustfmt.toml: set max_width = 100 explicitly, with a comment that the limit is the project's rule and why the overflow options are not set on stable. Also set any stable option that helps, for example use_small_heuristics if it keeps wrapping consistent; check that cargo fmt --all then changes nothing unexpected, or reformat in this one commit.
2. A check for what rustfmt cannot enforce: a workspace test, for example crates/rb-cli/tests/line_width.rs or a small tools/ script run by a test, that fails on any line over 100 columns in tracked .rs and .toml files. Count columns in chars, not bytes, and name each offending file:line. Exclude generated files, such as assets/ui/*.svg and target/, and say what else is excluded and why.
3. Fix the 31 existing lines: rewrap comments; split long strings with concat! or \ continuations.
4. Put the rule in AGENTS.md's standing rules: "lines at most 100 columns; cargo test checks it". GitHub Actions' ci.yml (task-108) runs cargo test, so it covers this automatically.
- Markdown docs (docs/*.md) keep their long table rows. Prose lines there are not checked by this task; say so in AGENTS.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 rustfmt.toml sets max_width = 100 with the reason the overflow checks are not enabled on stable; cargo fmt --all --check passes
- [ ] #2 A test fails on any tracked .rs/.toml line over 100 columns (naming file:line); the 31 existing lines are fixed and the test passes
- [ ] #3 AGENTS.md states the rule and the check; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->
