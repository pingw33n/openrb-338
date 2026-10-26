---
id: TASK-015
title: Rust workspace scaffold
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
labels:
  - engine
  - cli
milestone: m-1
dependencies: []
priority: high
type: chore
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Create the Rust workspace at the repository root: crates `rbs` (the RBS 4.2 file format), `rb-engine` (the audio engine), and `rb-cli` (the `rb` binary). Edition 2024, a rust-toolchain.toml pinning a stable toolchain, rustfmt and clippy settings, `cargo test` and `cargo clippy -- -D warnings` clean, a workspace README section in AGENTS.md (build and test commands). The engine crate's numerics rule goes into its crate docs: compute in f64, round to f32 exactly where the original stores a float, no FMA (`mul_add`), keep operand order (docs/initial_investigation.md section 4, "Numerics rules").
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 cargo build, cargo test and cargo clippy -- -D warnings pass on the empty crates
- [ ] #2 AGENTS.md has the build and test commands; the engine crate documents the numerics rule
<!-- AC:END -->
