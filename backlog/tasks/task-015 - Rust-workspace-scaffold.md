---
id: TASK-015
title: Rust workspace scaffold
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 16:12'
labels:
  - engine
  - cli
milestone: m-1
dependencies: []
priority: high
type: chore
ordinal: 8500
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Create the Rust workspace at the repository root: crates `rbs` (the RBS 4.2 file format), `rb-engine` (the audio engine), and `rb-cli` (the `rb` binary). Edition 2024, a rust-toolchain.toml pinning a stable toolchain, rustfmt and clippy settings, `cargo test` and `cargo clippy -- -D warnings` clean, a workspace README section in AGENTS.md (build and test commands). The engine crate's numerics rule goes into its crate docs: compute in f64, round to f32 exactly where the original stores a float, no FMA (`mul_add`), keep operand order (docs/initial_investigation.md section 4, "Numerics rules").
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 cargo build, cargo test and cargo clippy -- -D warnings pass on the empty crates
- [x] #2 AGENTS.md has the build and test commands; the engine crate documents the numerics rule
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Workspace Cargo.toml (resolver 3, edition 2024), three crates, rust-toolchain.toml pinned to 1.99.1 (installed stable), rustfmt/clippy config, numerics rules in rb-engine crate docs, build/test section in AGENTS.md.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-ad6d64f9f06bff67b

cargo build, cargo test (3 crates, 0 tests), cargo clippy --all-targets -- -D warnings, cargo fmt --check all clean on 1.99.1. Cargo.lock committed (binary crate).

Review round 1: reworded the transcendental-table bullet in rb-engine/src/lib.rs as a plan (task-019) and noted x87 extended precision; fmt/clippy clean; commit amended.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Workspace scaffold on branch worktree-agent-ad6d64f9f06bff67b: crates rbs, rb-engine, rb-cli (bin rb), toolchain 1.99.1, numerics rule in rb-engine docs, AGENTS.md Build and test section. build/test/clippy clean.
<!-- SECTION:FINAL_SUMMARY:END -->
