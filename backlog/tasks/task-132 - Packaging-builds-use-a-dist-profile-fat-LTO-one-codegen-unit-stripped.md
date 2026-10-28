---
id: TASK-132
title: 'Packaging builds use a dist profile: fat LTO, one codegen unit, stripped'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 19:57'
updated_date: '2026-10-28 19:57'
labels:
  - tools
milestone: m-4
dependencies:
  - TASK-110
references:
  - Cargo.toml
  - scripts/build.sh
  - .github/workflows/release.yml
  - docs/building.md
priority: medium
type: enhancement
ordinal: 151000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user decided (2026-10-28) that link-time optimization is for packaging only. Local cargo build --release stays as it is.

Measured on 2026-10-28 on this Mac (aarch64), with CARGO_PROFILE_RELEASE_* overrides:
| Binary | Today | lto = "fat", codegen-units = 1, strip = true |
|---|---|---|
| OpenRB-338 | 25.0 MB | 15.0 MB |
| rb | 6.7 MB | 5.4 MB |
The build took about 1m54s. opt-level = "s" would give 12.7 MB, but was rejected for the engine's speed, so keep opt-level 3. The workspace Cargo.toml has no [profile.release] today.

What:
- Add [profile.dist] to the workspace Cargo.toml: inherits = "release", lto = "fat", codegen-units = 1, strip = true. Leave panic as it is.
- scripts/build.sh, and everything that builds packages (package.sh, package-dmg.sh, macos-app.sh, build-all.sh, the Docker Linux path and release.yml's package jobs), build with --profile dist and take the binaries from target/<target>/dist/ (or target/dist/). Fix every path that assumes target/.../release/. The dev scripts (macos-dev-app.sh) keep release.
- Check that rb-ui's build.rs version or profile logic and anything else that reads PROFILE or OUT_DIR still behave correctly.
- Fidelity: with the dist profile, rb render must equal the reference corpus. Run the compare with a dist-built rb: cargo run --profile dist -p rb-cli -- compare --manifest tests/refs/manifest.json --mod-dir DIR, with the mods from . It must be 175 of 175. LTO shouldn't change floating-point results (no fast-math, no FMA), but this proves it.
- Record the before and after sizes of every package built locally (the macOS tar.gz and universal dmg, the Windows zips through cargo-xwin, and Linux through Docker if it is running), and update docs/building.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 [profile.dist] (fat LTO, codegen-units 1, strip) exists and every packaging path builds with it; local release builds are unchanged
- [ ] #2 A dist-built rb compares 175 of 175 equal against the corpus; package sizes before and after are recorded in the notes and docs/building.md
- [ ] #3 cargo +nightly fmt --check, clippy, cargo test --release, shellcheck and actionlint pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Correction to the description (a shell quoting slip dropped the command): the corpus mods come from: cargo run --profile dist -p rb-cli -- mod recrypt --iso "$RB_CD_ISO" -o DIR, then compare --mod-dir DIR.
<!-- SECTION:NOTES:END -->
