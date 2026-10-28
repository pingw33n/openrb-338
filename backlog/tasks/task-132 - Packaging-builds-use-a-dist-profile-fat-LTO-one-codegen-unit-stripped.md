---
id: TASK-132
title: 'Packaging builds use a dist profile: fat LTO, one codegen unit, stripped'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 19:57'
updated_date: '2026-10-28 20:21'
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
- [x] #1 [profile.dist] (fat LTO, codegen-units 1, strip) exists and every packaging path builds with it, by code; the Linux Docker path is not run (the Docker daemon was down) and the first release run checks it; local release builds are unchanged
- [x] #2 A dist-built rb compares 175 of 175 equal against the corpus; package sizes before and after are recorded in the notes and docs/building.md
- [x] #3 cargo +nightly fmt --check, clippy, cargo test --release, shellcheck and actionlint pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Baseline sizes of all packages and the dmg with the release profile. 2. Add profile.dist, build.sh builds with it, check build.rs, docs, workflows. 3. Rebuild everything, record sizes. 4. Corpus compare with dist-built rb, 175 of 175. 5. Update docs/building.md, fmt, clippy, tests, shellcheck, actionlint, one commit.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Correction to the description (a shell quoting slip dropped the command): the corpus mods come from: cargo run --profile dist -p rb-cli -- mod recrypt --iso "$RB_CD_ISO" -o DIR, then compare --mod-dir DIR.

Branch: worktree-agent-acffadbf2b99303b0

Baseline (release + strip=symbols) and dist sizes in docs/building.md, section The dist profile. Package sizes (bytes) before to after: aarch64-apple tar.gz 15417562 to 11888843; x86_64-apple tar.gz 15940752 to 12531940; universal dmg 34302700 to 26980214; aarch64-windows zip 15703374 to 12583557; x86_64-windows zip 16469940 to 13365072. OpenRB-338 aarch64-apple 21058864 to 14981536. Linux not built: docker info shows a client only, the daemon was not running. build.sh was the only script with a release path; package.sh and release.yml call it; macos-app.sh keeps a release ui-art for the icon (not shipped). build.rs reads no PROFILE. Corpus: target/aarch64-apple-darwin/dist/rb compare with mods from mod recrypt --iso: 175 of 175 equal. Checks: fmt, clippy, cargo test --release 583 pass 0 fail, shellcheck, actionlint all clean.

Review round 1 (text fixes): size claim corrected to packages 19 to 23 percent smaller (tar.gz 22.9 and 21.4, dmg 21.3, zip 19.9 and 18.9), OpenRB-338 binary 18 to 29 percent (macOS 26 to 29, Windows 18 to 20), rb 3 to 9 percent; AC 1 reworded (Linux Docker path not run). Corpus commands: target/aarch64-apple-darwin/dist/rb mod recrypt --iso <Rebirth 2.01 iso> -o DIR (4 recrypted), then target/aarch64-apple-darwin/dist/rb compare --manifest tests/refs/manifest.json --mod-dir DIR. Output tail: modapp/Fonda Workout: EQUAL; modapp/Metallicon City: EQUAL; modapp/RSI: EQUAL; 175 of 175 equal (exit 0).

Review round 2 (text fix): the OpenRB-338 range in the round 1 line and the final summary was macOS only; Windows binaries fall 18.4 and 19.8 percent, universal 27.2 percent. The commit message never stated that range.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added [profile.dist] (inherits release, fat LTO, codegen-units 1, strip) to Cargo.toml; scripts/build.sh builds with --profile dist and copies from target/<target>/dist/; every packaging path goes through build.sh; local release and macos-dev-app.sh unchanged. Packages are 19 to 23 percent smaller (universal dmg 34.3 to 27.0 MB, aarch64 tar.gz 15.4 to 11.9 MB), OpenRB-338 18 to 29 percent (macOS 26 to 29, Windows 18 to 20), rb 3 to 9 percent. The Linux Docker path was not run (no Docker daemon). A dist-built rb compares 175 of 175 equal. cargo test --release 583 pass; fmt, clippy, shellcheck, actionlint clean. docs/building.md updated.
<!-- SECTION:FINAL_SUMMARY:END -->
