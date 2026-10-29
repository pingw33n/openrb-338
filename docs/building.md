# Building and packaging

How orb (the CLI, crate `orb-cli`) and OpenRB-338 (the UI, crate `orb-ui`) are built for release on the six
targets, locally and in GitHub Actions (task-108). The installers and bundles (.dmg, Windows
installer, AppImage) and the draft release build on this (tasks 110 to 113; the .dmg is
described under "The macOS app and .dmg").

Neither program needs a Propellerhead file at build or run time: the standard mod is
`assets/Standard.rbm`, embedded in the binaries. No step here fetches any other ReBirth file.
The project is licensed GPL-3.0-only (`LICENSE` is the verbatim GPLv3 text; `license` in
`Cargo.toml` is `GPL-3.0-only`), and every package ships `LICENSE`. Nothing is signed (see
"What the system shows for an unsigned download").

## Targets

Every OS has an amd64 and an arm64 variant:

| Target | Package | Notes |
|---|---|---|
| `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` | `.tar.gz`, `.AppImage` | built on Ubuntu 22.04: glibc 2.35 or newer; the AppImage is described under "The AppImage" |
| `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc` | `.zip` | MSVC, not gnu: it is what Rust, eframe's wgpu and the runners use; `cargo-xwin` cross-builds it from a Mac or Linux |
| `x86_64-apple-darwin`, `aarch64-apple-darwin` | `.tar.gz` | two packages, no universal binary (the `.dmg` has the `lipo` of the two, below) |

A package is `OpenRB-338-<YYYY.MMDD.HHMM>-<target>.tar.gz|zip` holding a folder of the same
name with `orb` (`orb.exe`), `OpenRB-338` (`OpenRB-338.exe`), `LICENSE` and `README.txt`. The
binaries are built with `--locked` and the `dist` profile (below).

### The dist profile (task-132)

Decided by the user (2026-10-28): link-time optimization is for packaging only. `Cargo.toml` has

```toml
[profile.dist]
inherits = "release"   # opt-level 3 (opt-level "s" was rejected: the engine's speed), unwinding panics
lto = "fat"
codegen-units = 1
strip = true
```

`scripts/build.sh` (so `package.sh`, `build-all.sh`, `package-dmg.sh`, the Docker path and every
package job of `release.yml`) builds with `--profile dist` and takes the binaries from
`target/<target>/dist/`. `cargo build --release` and `scripts/macos-dev-app.sh` are unchanged
(`macos-app.sh` renders the icon with a release `ui-art`, which is not shipped). A dist build
takes minutes more than a release build (about 1.5 to 3 minutes per target here). Nothing reads
`PROFILE` (`crates/orb-ui/build.rs` uses `OUT_DIR` and the version variables only), so the profile
changes no behavior.

Fidelity: a dist-built `orb` (`target/aarch64-apple-darwin/dist/orb compare --manifest
tests/refs/manifest.json --mod-dir DIR`, the mods from `orb mod recrypt --iso "$ORB_CD_ISO" -o
DIR`) compares 175 of 175 corpus songs equal, as the release build does.

Sizes on this Mac (2026-10-28, bytes; release profile with `strip = symbols`, then dist):

| Target | Binary `orb` | Binary `OpenRB-338` | Package |
|---|---|---|---|
| aarch64-apple-darwin | 5,924,320 to 5,418,736 | 21,058,864 to 14,981,536 | .tar.gz 15,417,562 to 11,888,843 |
| x86_64-apple-darwin | 6,170,656 to 5,749,832 | 22,011,896 to 16,373,416 | .tar.gz 15,940,752 to 12,531,940 |
| universal-apple-darwin | 12,101,088 to 11,185,904 | 43,078,960 to 31,365,536 | .dmg 34,302,700 to 26,980,214 |
| aarch64-pc-windows-msvc | 5,666,816 to 5,509,120 | 21,507,072 to 17,240,576 | .zip 15,703,374 to 12,583,557 |
| x86_64-pc-windows-msvc | 6,229,504 to 6,064,128 | 24,596,480 to 20,065,792 | .zip 16,469,940 to 13,365,072 |

The Linux packages were not built (Docker was not running).

### The version is the build time

Decided by the user (2026-10-28): no package, file name, plist or text shows the `version` of
`Cargo.toml` (it stays, cargo needs one). The version of OpenRB-338 is its UTC build time to the
minute in one form, `YYYY.MMDD.HHMM` (decided 2026-10-28, task-133), from `SOURCE_DATE_EPOCH`
(default now; `scripts/package.sh`, `package-dmg.sh` and `build-all.sh` export it once,
release.yml takes it from `prepare`). `scripts/version.sh` prints it, and every place below shows
the same string for one `SOURCE_DATE_EPOCH`:

| Where | Form | Example |
|---|---|---|
| `scripts/version.sh` | `YYYY.MMDD.HHMM` | `2026.1028.1930` |
| file names | `OpenRB-338-<version>-<target>.<ext>` | `OpenRB-338-2026.1028.1930-universal-apple-darwin.dmg` |
| About box (`crates/orb-ui/src/version.rs`, made by `build.rs`) | `<version> (<hash>)` | `2026.1028.1930 (b472847)` |
| `README.txt` of a package | `OpenRB-338 <version> (<target>)` | `OpenRB-338 2026.1028.1930 (x86_64-pc-windows-msvc)` |
| macOS `CFBundleShortVersionString` and `CFBundleVersion` (at most three integers) | `<version>` | `2026.1028.1930` |
| macOS `OpenRBBuildTime` (the one place with seconds) | `YYYY-MM-DD HH:MM:SS UTC` | `2026-10-28 19:30:00 UTC` |
| the release tag (task-113) | `v<version>` | `v2026.1028.1930` |

The hash in the About box is the short commit hash; `dev-` before it marks a build that is not of
main, `-dirty` after it a work tree with uncommitted changes to tracked files (`(unknown)` without
git).

Ordering: `MMDD` and `HHMM` are zero-padded, so a later build compares higher as text and
component by component (`2026.0930.2359 < 2026.1001.0000`). The seconds are dropped, so two
builds in one minute have one version. `MMDD` is at most 1231 and `HHMM` at most 2359, each
below 65535, as an App Store validation would need (`YYYY.MMDD.HHMMSS` would not).

`CFBundleVersion` is the time too, not a commit count: it orders builds across checkouts and
branches, and the .dmg's name carries it. Neither `orb` nor `OpenRB-338` has a `--version` flag,
so neither prints the Cargo version. Later packages (the Windows installers, the AppImages, the
draft release) name themselves the same way.

A release tag names its build: `v2026.1028.1930` is 2026-10-28 19:30:00 UTC, so
`SOURCE_DATE_EPOCH=$(scripts/version.sh --epoch v2026.1028.1930)` (1793215800, seconds 0) gives
every package of that release the version `2026.1028.1930`. The tag matches
`^v[0-9]{4}\.[0-9]{4}\.[0-9]{4}$`; `--epoch` fails for anything else and for a date that does not
exist (month 13, hour 25).

One time per build: the version comes from `SOURCE_DATE_EPOCH` alone, in the binary
(`build.rs`), the plist, the file names and the tag. A plain `cargo build` without it takes the
time of the build script's last run, so a bundle made from it with `scripts/macos-dev-app.sh`
(which has its own `SOURCE_DATE_EPOCH`, default now) can show a different version in its plist
than the About box. For a dev bundle that matters, build and wrap with the same variable
(`SOURCE_DATE_EPOCH=$(date +%s) cargo build --release -p orb-ui && SOURCE_DATE_EPOCH=...
scripts/macos-dev-app.sh`), or read the version from the About box.

### What may ship

Decided by the user (2026-10-28): the built-in kit `assets/Standard.rbm` (the "Standard
ReBirth" drum samples), embedded in both binaries, may ship in the public release packages:
the `.tar.gz`, the `.zip`, the `.dmg`, and the later installer and AppImages.

On Linux, both `orb` and OpenRB-338 link `libasound.so.2` (ALSA, package `libasound2`), so even
`orb --help` fails without it. OpenRB-338 also loads X11 or Wayland and the GL or Vulkan driver at
run time (dlopen): on a bare Ubuntu 22.04 it needed `libxcursor1`, `libxrandr2`, `libxi6`,
`libxkbcommon-x11-0`, `libegl1` and a driver (`mesa-vulkan-drivers` for software Vulkan, or
Mesa's OpenGL and EGL drivers, `libgl1-mesa-dri` and `libegl-mesa0`; see "The AppImage"). The most
recent glibc symbol the binaries use is `GLIBC_2.35` (OpenRB-338) and `GLIBC_2.34` (orb), on both
architectures, so the minimum is glibc 2.35. Both Linux packages were smoke-tested in
`ubuntu:22.04` containers (`orb --help`, `orb info` on a shipped song, and OpenRB-338
`--screenshot` under xvfb, which wrote a PNG), amd64 and arm64.

The Windows build of OpenRB-338 is a GUI-subsystem exe (task-111), see "The Windows installer".

## Scripts (`scripts/`)

| Script | Does |
|---|---|
| `scripts/build.sh TARGET` | builds `orb` and `OpenRB-338` (profile `dist`) for the target into `dist/<target>/bin/` |
| `scripts/package.sh TARGET` | `build.sh`, then packs `dist/<target>/OpenRB-338-<YYYY.MMDD.HHMM>-<target>.tar.gz|zip` (`SKIP_BUILD=1` packs the existing `bin/`) |
| `scripts/build-all.sh [TARGET...]` | packages every target practical from this machine, and lists the skipped and failed ones |
| `scripts/package-dmg.sh` | builds both macOS targets, then `dist/universal-apple-darwin/OpenRB-338-<YYYY.MMDD.HHMM>-universal-apple-darwin.dmg` (`SKIP_BUILD=1` uses the existing `bin/` folders) |
| `scripts/package-installer.sh TARGET` | `build.sh` for a Windows target, then `dist/<target>/OpenRB-338-<YYYY.MMDD.HHMM>-<x64\|arm64>-setup.exe` with NSIS (`SKIP_BUILD=1` uses the existing `bin/`; `MAKENSIS` names makensis) |
| `scripts/installer/openrb.nsi`, `scripts/installer/path.ps1` | the installer's script, and the helper that edits the user's PATH |
| `scripts/package-appimage.sh TARGET` | `build.sh` for a Linux target, then `dist/<target>/OpenRB-338-<YYYY.MMDD.HHMM>-<x86_64\|aarch64>.AppImage` with appimagetool (`SKIP_BUILD=1` uses the existing `bin/`) |
| `scripts/appimage-pack.sh TOOL APPDIR OUT` | runs appimagetool (called by `package-appimage.sh`, in the build image off Linux; unpacks the tool with `unsquashfs` where the AppImage cannot be exec-ed) |
| `scripts/appimage-smoke.sh APPIMAGE` | runs an AppImage in a stock `ubuntu:22.04` container: contents, no glibc bundled, `orb`, `--screenshot` under xvfb |
| `scripts/macos-app.sh OUT_DIR UI [CLI]` | makes `OUT_DIR/OpenRB-338.app` from the binaries (Info.plist, icon, ad-hoc signature) |
| `scripts/macos-dev-app.sh` | wraps `target/release/OpenRB-338` into `target/OpenRB-338.app` for everyday runs |
| `scripts/version.sh` | prints the version `YYYY.MMDD.HHMM` (the UTC build time from `SOURCE_DATE_EPOCH`, to the minute); `--time` the full time, `--epoch v2026.1028.1930` the `SOURCE_DATE_EPOCH` of a release tag |
| `scripts/release-env.sh` | prints the build environment all packages of a release share (`SOURCE_DATE_EPOCH`, `ORB_BUILD_BRANCH`) |
| `scripts/docker/linux-build.Dockerfile` | the Linux build image (Ubuntu 22.04, build tools, ALSA headers) |

`dist/` is git-ignored. The later packaging scripts take `dist/<target>/bin/` as their input.

The version string (Help > About, task-102) is made by `crates/orb-ui/build.rs` from two
environment variables that the scripts pass through, including into Docker: `SOURCE_DATE_EPOCH`
(the build time; default now) and `ORB_BUILD_BRANCH` (`main` makes the hash the main form; a
tag's detached HEAD needs it). Example: `SOURCE_DATE_EPOCH=1793181022 ORB_BUILD_BRANCH=main
scripts/package.sh aarch64-apple-darwin`.

### What each host builds

| Host | Builds | How |
|---|---|---|
| macOS (this Mac, arm64) | `aarch64-apple-darwin`, `x86_64-apple-darwin` | `cargo build --target` (Apple's linker cross-links both) |
| | both Windows targets | `cargo xwin` |
| | both Linux targets | Docker (`--platform linux/arm64` is native on Apple silicon; `linux/amd64` is emulated and slow) |
| Linux | its own architecture | `cargo build`; the other architecture through Docker with qemu emulation (slow; a runner is easier) |
| Windows | its own architecture, and the other Windows one | `cargo build` in Git Bash |

`scripts/build-all.sh` skips what is not practical here and says why: Linux without a running
Docker, Windows without `cargo-xwin`, macOS off a Mac. What cannot be cross-built from a Mac:
the Linux binaries (cpal links `libasound`, so ALSA headers and a Linux linker are needed:
hence Docker; cargo-zigbuild does not help for the ALSA part) and the macOS binaries off macOS
(they need Apple's SDK). Windows cross-builds without Windows because `cargo-xwin` downloads the
Microsoft CRT and SDK (accepting Microsoft's license, `XWIN_ACCEPT_LICENSE=1`, which the script
sets) into the cargo-xwin cache, `~/Library/Caches/cargo-xwin` on macOS (`~/.cache/cargo-xwin`
on Linux).

### Installing the tools (this Mac)

These are the commands that were run (Homebrew and rustup; nothing else is changed):

```sh
brew install llvm             # clang-cl, lld-link and llvm-lib, for the C parts of the Windows builds
brew install actionlint shellcheck   # to check the workflows and the scripts
brew install makensis         # NSIS 3.13, for the Windows installers (task-111)
brew install sevenzip         # `7zz l setup.exe` lists an installer's files (the check below)
cargo install cargo-xwin --locked
rustup target add --toolchain 1.99.1 x86_64-apple-darwin x86_64-pc-windows-msvc aarch64-pc-windows-msvc
```

`build.sh` adds the target of the build itself (`rustup target add`), so the last line is only a
way to fetch them ahead. `clang-cl` must be on `PATH` (Homebrew's `/opt/homebrew/opt/llvm/bin`;
Swift's toolchain also provides one). Docker Desktop must be running for the Linux builds; the
scripts do not start it. The Windows C build sets `CL=-Wno-incompatible-pointer-types`: the
vendored libvorbis (`aotuv_lancer_vorbis_sys`) passes a `float *` to `_mm_prefetch`, which MSVC's
`cl` accepts with a warning and clang-cl 21 and newer rejects (cargo-xwin overwrites `CFLAGS_*`
and `CC_*`, so the variable `CL`, which clang-cl reads itself, is the way in). The same Homebrew
LLVM gives `llvm-rc`, which compiles the exe's icon and version resource when cross-building for
Windows (below); `llvm-readobj` (also there) reads the result back.

### Docker (Linux builds off Linux)

`build.sh` builds the image `openrb-linux-build:<arch>` and runs itself inside it with the
checkout mounted at its own path (and the git common dir read-only, for a worktree's `.git`
file, so the version string has its hash). The Rust toolchain pinned in `rust-toolchain.toml`,
the cargo registry and the target dir are in the named volumes `openrb-rustup-<arch>`,
`openrb-cargo-<arch>` and `openrb-target-<arch>`, so a second run is incremental.
`docker volume rm` them to start clean.

## Environment variables

Every variable of the project has the prefix `ORB_` (task-068; the CLI is `orb`). The old `RB_`
and `OPENRB_` names are not read any more and have no deprecated alias.

| Variable | Read by | Meaning | Old name |
|---|---|---|---|
| `ORB_CD_ISO` | `orb-testkit` (the gated tests; the `orb` commands take the image as `--iso`) | the ReBirth 2.01 CD image; the tests skip when it is unset | `RB_CD_ISO` |
| `ORB_THIRDPARTY_MODS_DIR` | `orb-testkit` | the user's folder of RECRYPTED third-party mods, read in place | `RB_THIRDPARTY_MODS_DIR` |
| `ORB_THIRDPARTY_SONGS_DIR` | `orb-testkit` | the user's folder of songs, read in place | `RB_THIRDPARTY_SONGS_DIR` |
| `ORB_MOD_DIR` | `orb render`, `compare`, `play`, `info`; OpenRB-338 | the folder of recrypted `.rbm` mods (`--mod-dir` wins) | `RB_MOD_DIR` |
| `ORB_RB20FUL` | `orb mod recrypt` | the path of `RB20FUL.DAT` (`--rb20ful` wins); also the default of `tools/rbcorpus.py` | `RB_RB20FUL` |
| `ORB_UI_CONFIG` | OpenRB-338 | the settings file to use instead of `OpenRB-338/ui.toml` in the configuration folder | `RB_UI_CONFIG` |
| `ORB_GPU_INFO` | OpenRB-338 | `1` prints the graphics adapter wgpu chose (`scripts/appimage-smoke.sh`) | `OPENRB_GPU_INFO` |
| `ORB_BUILD_BRANCH` | `crates/orb-ui/build.rs`, `scripts/release-env.sh`, `scripts/build.sh` (passed into Docker), `release.yml` | `main` or `other`: makes the About box's hash the main form | `RB_BUILD_BRANCH` |
| `ORB_UI_VERSION` | set by `crates/orb-ui/build.rs` for rustc (`env!`), never by hand | the version string of the About box | `RB_UI_VERSION` |
| `ORB_APPIMAGE_EMULATED` | `scripts/package-appimage.sh`, `appimage-pack.sh`, `appimage-smoke.sh` | `1` on an x86_64 AppImage built or checked on a Mac (local only) | `OPENRB_APPIMAGE_EMULATED` |
| `ORB_PYTHON` | `tests/common/python.rs` (the tests that run `tools/*.py`) | the Python 3 program or command | `RB_PYTHON` |
| `ORB_LIVE_OUT` | `orb-player`'s `live_messages` test | an output folder for the session files rbhost replays | `RB_LIVE_OUT` |
| `ORB_REBIRTH_DLL` | `tools/gen_static_tables.py` | the path of `ReBirth.dll` | `RB_REBIRTH_DLL` |

Removed instead of renamed, because nothing reads them: `RB_RBM_DIR` (the deprecated alias of
`RB_MOD_DIR`), `RB_STANDARD_MOD` (the standard-mod lookup went with task-067; until task-068 one test
still set it, to check that it is ignored), `RB_MOD_ORIG_DIR`, `RB_CD_REBIRTH_EXE` and `RB_THIRDPARTY_DIR`
(named in old notes only). Not renamed: `RBM_TEST_DAT` and `RBM_TEST_MODS` (a cargo test sets them
for `tools/rbm_test.py` only; `RBM` is the mod format), and the variables of other tools
(`SOURCE_DATE_EPOCH`, `GITHUB_REF_NAME`, `WGPU_BACKEND`). The settings folder is `OpenRB-338`
(`OpenRB-338/ui.toml`); settings of the old `rb/` folder are not carried over (no migration).

## GitHub Actions (`.github/workflows/`)

Decisions of the user (2026-10-28): the cost of six release package jobs and the three-OS CI
on the private repository is accepted; accepting Microsoft's SDK and CRT license through
`XWIN_ACCEPT_LICENSE=1` for the local cargo-xwin builds is approved; no signing; the project is
GPL-3.0-only.

Actions are pinned to a commit SHA (the version in a comment). No workflow uses a secret.

- `ci.yml`, on every push and pull request:
  - `fmt`: `cargo +nightly-2026-10-18 fmt --all --check`. `rustfmt.toml` has unstable options,
    so formatting needs a nightly; this one is pinned and installed with
    `rustup toolchain install nightly-2026-10-18 --profile minimal --component rustfmt`.
    Build, clippy and tests use the stable toolchain pinned in `rust-toolchain.toml`.
  - `test` on `ubuntu-latest`, `windows-latest` and `macos-latest`:
    `cargo clippy --workspace --all-targets --locked -- -D warnings` and
    `cargo test --workspace --locked --no-fail-fast` (one run shows every failure). Some tests
    run `tools/*.py` with Python 3 (every runner has it): `ORB_PYTHON` if set, else the first of
    `python3`, `python`, `py -3` on PATH that is Python 3 (`tests/common/python.rs`); none found
    fails the test with a message naming `ORB_PYTHON`. The tests that need `ORB_CD_ISO`,
    `ORB_THIRDPARTY_MODS_DIR` or `ORB_THIRDPARTY_SONGS_DIR` (the only external inputs of the
    tests; AGENTS.md) skip when those are absent (checked locally with all of them unset). The
    synthetic recrypt tests (`crates/rbm/tests/recrypt_python.rs`, which run `tools/rbm.py`)
    need none of them.
    Linux installs `libasound2-dev` first.
- `release.yml`, on a tag `v*` and by hand:
  - `prepare` computes one `SOURCE_DATE_EPOCH` and one `ORB_BUILD_BRANCH` (`scripts/release-env.sh`)
    so all packages agree. On a tag the build time is the tag's and a tag that is not
    `v<YYYY.MMDD.HHMM>` fails here, which skips every other job.
  - `package`, one job per target, each on a native runner: `ubuntu-22.04` (x86_64) and
    `ubuntu-22.04-arm` (aarch64) for Linux (the oldest LTS image: glibc 2.35); `windows-latest`
    and `windows-11-arm`; `macos-15-intel` (amd64) and `macos-15` (arm64). Each runs
    `scripts/package.sh <target>` and uploads the package (`package-<target>`) and the binaries
    (`bin-<target>`, one day) as workflow artifacts.
  - The runner images change: GitHub retires old ones (macos-13 is gone, which is why amd64
    macOS uses `macos-15-intel`). Fix a label here when a run reports one as missing.
  - The Windows arm64 job installs rustup if the image has none.
  - `dmg` (task-110), on `macos-15` after the package jobs: downloads the two macOS `bin-<target>`
    artifacts, runs `SKIP_BUILD=1 scripts/package-dmg.sh` and uploads the artifact `dmg`
    (`OpenRB-338-<YYYY.MMDD.HHMM>-universal-apple-darwin.dmg`). It reuses the two package
    builds and the `prepare` build time, so the app's version string is theirs.
  - `installer` (task-111), one job per Windows architecture on `ubuntu-latest` after the package
    jobs: installs NSIS (`apt-get install nsis`), downloads `bin-x86_64-pc-windows-msvc` or
    `bin-aarch64-pc-windows-msvc`, runs `SKIP_BUILD=1 scripts/package-installer.sh <target>` and
    uploads the artifact `installer-x64` or `installer-arm64`
    (`OpenRB-338-<YYYY.MMDD.HHMM>-<x64|arm64>-setup.exe`). It needs no Windows runner and no
    Rust, and takes the `prepare` build time, so the installer's version is the exe's. Not run
    yet: only `actionlint` and a local run of the same script checked it (see "The CI job and
    old NSIS" below; the first release run is the check, task for it is separate).
  - `appimage` (task-112), one job per Linux architecture after the package jobs, on
    `ubuntu-22.04` (x86_64) and `ubuntu-22.04-arm` (aarch64): downloads `bin-<target>`, runs
    `SKIP_BUILD=1 scripts/package-appimage.sh <target>`, then `scripts/appimage-smoke.sh` on the
    result (a stock `ubuntu:22.04` container), and uploads the artifact `appimage-x86_64` or
    `appimage-aarch64` (`OpenRB-338-<YYYY.MMDD.HHMM>-<x86_64|aarch64>.AppImage`). It takes the
    `prepare` build time. Not run yet (the first release run is the check, task-134).
  - `release` (task-113), "draft release", only on a tag, after all the jobs above: see "Cutting a
    release" below. It is the one job with `contents: write`; the workflow's own permission is
    `contents: read`.

Validation before merge: `actionlint .github/workflows/*.yml` and `shellcheck scripts/*.sh`
(both from Homebrew). `act` was not used: it cannot run the macOS and Windows runners.

### Cutting a release (task-113)

The tag names the release moment and the user chooses it: `v` + the version of the packages,
`v<YYYY.MMDD.HHMM>`, the UTC date and time to the minute (`v2026.1028.1930` is 2026-10-28
19:30 UTC; "The version is the build time"). The packages' own build time is the tag's
(`SOURCE_DATE_EPOCH` from `scripts/version.sh --epoch TAG`, seconds 0), so every binary and file
name of the release carries the tag's version. Tag the commit to release, from the time of the
tag (UTC), on or off main:

```
git tag "v$(date -u +%Y.%m%d.%H%M)"        # e.g. v2026.1028.1930
git push origin v2026.1028.1930
```

- Preview the changelog first: `scripts/changelog.sh v2026.1028.1930` (the tag, or `HEAD` before
  tagging).
- The push starts `release.yml`. `prepare` checks the tag against
  `^v[0-9]{4}\.[0-9]{4}\.[0-9]{4}$` (and that the date exists); another tag (`v1`, `vfoo`) fails
  there with a message, the other jobs are skipped and no release is created.
- When `package`, `dmg`, `installer` and `appimage` all pass, the `release` job:
  1. downloads `package-*`, `dmg`, `installer-*` and `appimage-*`;
  2. `scripts/release-assets.sh` checks that exactly the 11 files of a release are there (four
     `.tar.gz`, two `.zip`, the `.dmg`, the two `-setup.exe`, the two `.AppImage`, all named with
     the tag's version) and writes `SHA256SUMS.txt` over them (`sha256sum -c` form, sorted);
     a missing or foreign file fails the job, so a release never ships without a package;
  3. `scripts/changelog.sh` writes the release body;
  4. creates a draft release with `gh release create --draft --verify-tag`, titled
     `OpenRB-338 <version>`, with the 11 files and `SHA256SUMS.txt` attached (12 assets).
- It never publishes. The release is a draft that only the user can see: read the body and the
  assets on the Releases page, then press "Publish release" (or `gh release edit TAG --draft=false`).
  Nothing is signed (Gatekeeper and SmartScreen: "What the system shows for an unsigned
  download").
- Running the workflow again for the same tag (Re-run jobs, or "Run workflow" on the tag) deletes
  the draft of that tag and creates it anew: new body, new assets, still one release. A release
  already published is never touched: the job fails with a message.
- Where the tagged commit is on `origin/main`, the binaries show the hash without `dev-`
  (`ORB_BUILD_BRANCH=main`, `scripts/release-env.sh`); a tag on another branch shows `dev-`.
- `workflow_dispatch` ("Run workflow" on a branch) builds all the artifacts (on the run's page) with the build time now, and creates no release:
  the `release` job runs only when the ref is a tag. Run on a tag it does what the tag push does.

`scripts/changelog.sh [--since REV] [PREV] NEW` writes the body: the commits of `PREV..NEW`,
without merge commits and without `backlog:` ones, grouped under `engine`, `cli`, `tools`,
`tests`, `docs` (then `other` for a subject with no such prefix), each as the subject without its
prefix and the task ids of its `Task:` lines in parentheses. Without `PREV` it is the greatest
earlier tag of the release form (text order is version order; `vfoo` and `v2026.1028.1500x` are
ignored). The first release has no earlier tag: the whole history is listed, and the first line
says so; `--since REV` lists the commits after REV instead. Example (a scratch clone with fake
tags):

```
Changes since v2026.1028.1500 up to v2026.1029.0900: 15 commits, 9 listed (backlog commits are left out).

### tools

- Linux AppImage of OpenRB-338 (x86_64, aarch64), release.yml job (task-112)
- Windows NSIS installers, GUI-subsystem UI exe with icon (task-111)
```

Not run on GitHub yet: the `release` job needs a pushed tag; the checks are task-134.

## The macOS app and .dmg (task-110)

Double-clicking a bare binary in Finder opens Terminal, and the Dock and Activity Monitor show
the generic executable icon for it (the icon comes from the bundle's `CFBundleIconFile`; eframe
only sets the Dock icon at run time). The `.app` bundle fixes both.

`scripts/package-dmg.sh` (Mac only) builds
`dist/universal-apple-darwin/OpenRB-338-<YYYY.MMDD.HHMM>-universal-apple-darwin.dmg`:

- `OpenRB-338.app`, made by `scripts/macos-app.sh`: `Contents/MacOS/OpenRB-338` is the UI as a
  universal binary (`lipo` of the aarch64 and x86_64 builds; `lipo -archs` says `x86_64 arm64`),
  `Contents/Resources/OpenRB-338.icns` is the `app_icon` part rendered at 16 to 1024 px
  (`cargo run -p orb-ui-art --bin ui-art -- iconset DIR`, DIR required, with the same resvg as
  the panel, then `iconutil`), and `Contents/Info.plist` has `CFBundleName` and
  `CFBundleDisplayName` OpenRB-338, the identifier `net.emphased.openrb338` (the user's choice,
  2026-10-28), `CFBundleShortVersionString` and `CFBundleVersion` = the build time as
  `YYYY.MMDD.HHMM` (see "The version is the build time"), `OpenRBBuildTime`,
  `LSMinimumSystemVersion` = the highest `minos` of the binary (`scripts/build.sh` sets
  `MACOSX_DEPLOYMENT_TARGET=11.0` for both macOS targets, so both slices need macOS 11.0;
  `macos-app.sh` fails if it finds no `minos`), `NSHighResolutionCapable` and
  `LSApplicationCategoryType` `public.app-category.music`.
- The CLI `orb` is inside the bundle, `Contents/MacOS/orb`, also universal. Why inside and not next to the app: dragging the app to Applications installs both
  and nothing is left behind, the bundle's seal covers both, and an update replaces both.
  The cost is a longer path; `README.txt` in the image gives
  `sudo ln -s /Applications/OpenRB-338.app/Contents/MacOS/orb /usr/local/bin/orb`.
- An `Applications` link, `LICENSE` and `README.txt` (install, Gatekeeper, the `orb` link). There
  is no background picture or icon layout (that needs Finder scripting, which is not
  reproducible on a runner).
- Signing is ad hoc only (Apple silicon refuses an unsigned bundle; no Developer ID, no
  notarization): `orb` is signed explicitly (`codesign --force -s -`; before, only the linker had
  signed it on its own), then the bundle without `--deep`, whose seal covers both executables,
  then `codesign --verify --deep --strict`.
- Reproducibility: `cp -p` and `touch -t` from `SOURCE_DATE_EPOCH` fix the times of the bundle
  and the image's files. Two runs of `SKIP_BUILD=1 scripts/package-dmg.sh` give images whose
  mounted files are byte-identical (the same sha256 for every file), but the `.dmg` files
  themselves differ (`cmp`: the header; hdiutil writes a new volume UUID and creation date),
  and hdiutil has no option for a fixed one, so the image is not byte-reproducible.

`scripts/macos-dev-app.sh` is for everyday runs: after `cargo build --release -p orb-ui` it copies
`target/release/OpenRB-338` into `target/OpenRB-338.app` (same Info.plist, icon, ad-hoc
signature); `open target/OpenRB-338.app`. Run it again after each build.

How a bundled app differs, from `lsappinfo list` for the launched copy and for the bare binary:
the bare binary has `bundleID=[ NULL ]`, `bundle path` = the binary itself and no version, so
Activity Monitor has no icon file to show; the app has `bundleID="net.emphased.openrb338"`,
its `.app` as `bundle path` and its `Version` (the build time), which is where Activity Monitor and the Dock
read the name and the icon.

In CI the `dmg` job of `release.yml` runs `SKIP_BUILD=1 scripts/package-dmg.sh` over the two
macOS `bin-<target>` artifacts, so the app carries the same version string as the `.tar.gz`
packages.

## The Windows installer (task-111)

`scripts/package-installer.sh x86_64-pc-windows-msvc|aarch64-pc-windows-msvc` builds
`dist/<target>/OpenRB-338-<YYYY.MMDD.HHMM>-x64-setup.exe` or `...-arm64-setup.exe` (names as the
user decided, 2026-10-28; task-133 for the version) with NSIS: `scripts/installer/openrb.nsi`.
NSIS rather than Inno Setup because `makensis` runs on macOS, Linux and Windows, so the same
script builds on this Mac and on a CI runner (Inno Setup needs Windows or Wine). The installer
itself is a 32-bit x86 program (the NSIS stub), which Windows on Arm runs under emulation; what
it installs is the native x64 or arm64 build.

**What it installs.** Per user, to `%LOCALAPPDATA%\Programs\OpenRB-338` (`/D=DIR` changes it):
`OpenRB-338.exe`, `orb.exe`, `LICENSE.txt` (the GPLv3, also the
installer's license page) and `README.txt`, and `Uninstall.exe`. Per user, not per machine,
because the program is two executables with no service, driver or shared file; because an
unsigned installer that also asks for administrator rights shows a second warning for nothing
(no UAC prompt now); and because a per-user PATH needs no rights either. The cost: other
accounts of the PC do not get the program (they run the installer themselves). The pages:
welcome, license, components, folder, install, finish (start the app, show the readme).

| Component | Default | Does |
|---|---|---|
| OpenRB-338 and orb | required | the files above; the uninstaller; the entry in Settings > Apps > Installed apps (Add or remove programs) under `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\OpenRB-338`: `DisplayName`, `DisplayVersion` = `YYYY.MMDD.HHMM`, `Publisher`, `DisplayIcon`, `InstallLocation`, `UninstallString`, `QuietUninstallString`, `EstimatedSize`, no Modify or Repair |
| Start menu shortcut | on | `OpenRB-338` in the user's Start menu |
| Desktop shortcut | off | `OpenRB-338` on the desktop |
| Add orb to the PATH of the current user | off | appends the folder to `HKCU\Environment\Path` |

Silent install: `setup.exe /S [/STARTMENU=0|1] [/DESKTOP=0|1] [/PATH=0|1] [/D=DIR]` (`/D=` must
be the last argument and unquoted, an NSIS rule). Silent uninstall: `Uninstall.exe /S`, which
returns at once (it copies itself to a temporary folder to delete its own file); add `_?=DIR`
(`Uninstall.exe /S _?=C:\dir`) to wait for it, which then leaves `Uninstall.exe` behind to
delete by hand.

**The PATH.** `scripts/installer/path.ps1` does it (the installer runs it with
`powershell -NoProfile -ExecutionPolicy Bypass -File`): NSIS strings are 1024 characters here
(`makensis -HDRINFO`: `NSIS_MAX_STRLEN=1024`), so a long PATH would be cut, and .NET's
`SetEnvironmentVariable` turns the `REG_EXPAND_SZ` value into `REG_SZ`, so `%USERPROFILE%` in it
would stop expanding. The script reads the value unexpanded and writes `REG_EXPAND_SZ` back,
keeps every entry that is not ours exactly as it is (empty ones too), adds the folder once
(after a `;` unless the value is empty or ends with one), and the uninstaller removes our entry
and the `;` after it (a last entry: only the entry), so install then uninstall gives the value
back, except that a value without a `;` at its end ends up with one; then the installer
broadcasts `WM_SETTINGCHANGE`, so new
terminals see it (an open one does not). If PowerShell fails (a group policy that forbids
scripts even with `Bypass`), the installer says so and goes on. The uninstaller runs `remove`
whenever `path.ps1` is in the folder.

**The uninstaller** removes the files, the shortcuts, the PATH entry and both registry keys, and
the folder if it is empty. It keeps the user's own files in the folder, and the settings file
(`%APPDATA%\OpenRB-338\ui.toml`) and songs.

**Running programs, upgrades.** Before it changes anything, the installer and the uninstaller try
to open `OpenRB-338.exe` and `orb.exe` of the folder for writing, which fails while one runs from
it. Then a box asks to close the program (Retry or Cancel); under `/S`, or on Cancel, nothing is
changed and the exit code is 5 (tested on the test machine for both). If a file still cannot be
deleted after that (it appeared meanwhile), the uninstaller keeps the entry in Apps and the
registry keys, says so, and exits 6 (not exercised), so the uninstall can be repeated.
Installing over an
earlier version into the same folder just overwrites the files and the registry values (the
folder is remembered in `HKCU\Software\OpenRB-338` and is the default). Installing into another
folder removes the PATH entry of the earlier folder (a new one is added only if the PATH
component is ticked), moves the Apps entry to the new folder and leaves the earlier folder's
files, which can be deleted by hand. The shortcuts are overwritten. Start menu and desktop are
the current user's (`SetShellVarContext current`).

**The GUI subsystem.** `crates/orb-ui/src/main.rs` has
`#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]`: the release
and dist builds of `OpenRB-338.exe` open no console window; a debug build keeps one. `orb.exe`
(crate `orb-cli`) is unchanged, a console program. What a GUI-subsystem program loses:
`eprintln!` has nowhere to go. `attach_parent_console` (same file) attaches to the console of the
program that started it (`cmd`, PowerShell) with `AttachConsole` and points stdout and stderr at
`CONOUT$`, unless they were redirected (then they stay), so the usage text, a song that does not
open and the audio error show in a terminal. Still lost: `cmd` and PowerShell do not wait for a
GUI program, so the prompt comes back at once and the text appears over it (`start /wait
OpenRB-338.exe`, or `| more`, waits); started from the Start menu or Explorer, the text goes
nowhere (the errors that matter also show in the app's message box). `--screenshot OUT.png`
writes a file and is not affected.

**The icon and the version resource** (Windows targets only). `assets/app_icon.ico` is the
`app_icon` part of `assets/ui` rendered by `cargo run --release -p orb-ui-art --bin ui-art -- ico`
(same resvg as the panel and the macOS iconset): 16, 24, 32, 48, 64, 96, 128 and 256 pixels, PNG
entries. `crates/orb-ui/build.rs` writes an `.rc` with that icon and a `VERSIONINFO` (file and
product version `YYYY.MMDD.HHMM` as the four 16-bit numbers 2026, 1028, 1930, 0 and as text;
product name OpenRB-338, license GPL-3.0-only) and `embed-resource` compiles it (`llvm-rc` from
Homebrew LLVM on this Mac, `rc.exe` on a Windows runner) into the binary of `orb-ui` only, only
when the target is Windows. A failure fails the build. The test
`ico_has_the_sizes_and_the_committed_file_matches` (`crates/orb-ui-art/tests/art.rs`) checks the
committed file's sizes; the pixels are not compared byte for byte (two CPUs can round the last
bit differently), so regenerate the file when `app_icon.svg` changes.

`setup.exe` and `Uninstall.exe` have the same icon (`MUI_ICON`, from `assets/app_icon.ico`).

**The CI job and old NSIS.** `windows-latest` has no NSIS (the runner-images list has Inno Setup
and WiX, not NSIS), so the job runs on Ubuntu, whose apt NSIS is older than the 3.13 checked
here (3.08 on ubuntu-24.04, as reviewed). The script uses nothing that is new in 3.13 as far
as I can tell from reading it (`Unicode true`, MUI2, FileFunc, LogicLib `${ForEach}`,
`nsExec`, `SendMessage /TIMEOUT`, `ManifestDPIAware` all exist in 3.0x), and the job sets
`MAKENSIS_FLAGS` empty (no `-WX`), so a warning of the older makensis cannot stop a release. It
was not run on 3.08: the first release run is the check.

**Reproducible.** NSIS 3.13 honors `SOURCE_DATE_EPOCH` (`makensis -HDRINFO` lists
`NSIS_SOURCE_DATE_EPOCH`; the header time), and the script
copies the files into a stage folder with the build time as their mtime (NSIS stores it), so two
runs over the same binaries give byte-identical installers (checked here, both architectures).

**The installer is a PE of its own:** `file` says `PE32 executable (GUI) Intel 80386, Nullsoft
Installer self-extracting archive`, `7zz l` lists its files.

### Checked (2026-10-28, this Mac and the Windows test machine)

The command line tool was still called `rb` (`rb.exe`) in these runs; task-068 renamed it `orb`.

With `SOURCE_DATE_EPOCH=1793215800` (version `2026.1028.1930`):

| File | Bytes |
|---|---|
| `OpenRB-338-2026.1028.1930-x64-setup.exe` | 8,395,465 |
| `OpenRB-338-2026.1028.1930-arm64-setup.exe` | 7,498,263 |

- Local: `makensis -V2 -WX` (warnings are errors) builds both; `7zz l` lists `OpenRB-338.exe`,
  `rb.exe`, `LICENSE.txt`, `README.txt`, `path.ps1` and the uninstaller; `llvm-readobj
  --file-headers` says `IMAGE_SUBSYSTEM_WINDOWS_GUI` for `OpenRB-338.exe` and
  `IMAGE_SUBSYSTEM_WINDOWS_CUI` for `rb.exe` (`file`: `PE32+ executable (GUI)` and
  `(console)`), machine AMD64 and ARM64; `llvm-readobj --coff-resources` lists 8 `RT_ICON`, the
  `GROUP_ICON` and the `VERSIONINFO` (file version 2026.1028.1930.0).
- On the Windows test machine (Windows 11 build 26200, x64; folder `C:\re\rb\inst111`, never
  the ReBirth installation, deleted afterwards): `setup.exe /S /PATH=1 /DESKTOP=1
  /D=C:\re\rb\inst111\app` exits 0, the six files are there, the `Uninstall` registry key has the
  name and version `2026.1028.1930`, both shortcuts exist, PATH is still `REG_EXPAND_SZ`
  with the folder appended and the old entries intact; the file properties show the version and
  product name; `rb.exe --help` prints; `OpenRB-338.exe` starts (window title `OpenRB-338 -
  Untitled`, the panel in a screenshot) with no child process (no console host);
  `OpenRB-338.exe --bogus` with stderr redirected to a file writes the usage text and exits 2,
  and started through ShellExecute from PowerShell it exits 2. `Uninstall.exe /S _?=...` exits
  0 and removes the registry keys, both shortcuts and the PATH entry (PATH equal to before),
  leaving only `Uninstall.exe`. The first uninstall, by the first draft of `path.ps1`, also
  dropped an empty entry at the end of PATH; the script was changed to keep other entries
  exactly and the cycle repeated. The whole cycle was run again with the final x64 installer
  (sha256 `374d8097...5267`) after the review round: install, `rb --help`, app start, then
  `setup.exe /S` over the running app (exit 5, nothing changed), `Uninstall.exe /S` of the
  running app (exit 5, files and the Apps entry kept), an install in place over the closed app
  (exit 0), an install into a second folder (the PATH entry of the first folder is gone, the
  Apps entry points to the second), and uninstalls; the PATH value was byte-identical to its
  start. The exit-6 path (a file that cannot be deleted) was not exercised.
- `assets/app_icon.ico` and a fresh `ui-art ico` on this host (aarch64 macOS) are byte-identical
  (`cmp`, sha256 `67f66747...c366e7`); other CPUs may differ in the last bit of a pixel, which is
  why the test compares sizes only.
- The icon path in `build.rs` is not canonicalized (a native Windows `\\?\` path may not open in
  `rc.exe`). The native-Windows build (`windows-latest`, `windows-11-arm`: `ci.yml`, the release
  package jobs, with `rc.exe`) was not run here and is unverified until a CI Windows job runs;
  the cross build with `llvm-rc` was.
- Not checked: the arm64 installer was not run (the machine is x64); `AttachConsole`'s visible
  output was not seen (the test machine's console cannot be read back through rcs; the code runs
  without a crash and does not change the exit code); the installer's own wizard pages were not
  clicked through (only `/S`); the `installer` jobs of `release.yml` ran nowhere (the first
  release run checks them, with Ubuntu's older makensis).

## The AppImage (task-112)

One file per architecture, `OpenRB-338-<YYYY.MMDD.HHMM>-x86_64.AppImage` and `-aarch64.AppImage`
(the version is the build time, `scripts/version.sh`), that runs on most distributions without
installing: `chmod +x` it and run it. Not signed (decided 2026-10-28). The `.tar.gz` of task-108
stays as the plain form.

**Build.** `scripts/package-appimage.sh <target>` takes `dist/<target>/bin/` (the `dist`-profile
binaries, built by `scripts/build.sh` in the Ubuntu 22.04 image or on the 22.04 runner), lays out
an AppDir and packs it with appimagetool. The tool is pinned: appimagetool 1.9.1, downloaded from
its GitHub release into `dist/tools/` and checked against the sha256 in the script
(`ed4ce84f...` for x86_64, `f0837e74...` for aarch64; both equal the digests GitHub shows for the
release assets). It runs with `--appimage-extract-and-run`, so no FUSE is needed (CI runners and
containers have none). The packing runs natively on a Linux host of the target's architecture
and in the `openrb-linux-build:<arch>` Docker image anywhere else, so on this Mac it needs a
running Docker (the script does not start it; the other architecture is emulated and slow). The
icons are rendered on the host by `ui-art iconset` (cargo; resvg), so the host needs the Rust
toolchain. The AppDir:

| Path | Contents |
|---|---|
| `AppRun` | runs `usr/bin/OpenRB-338`; runs the bundled CLI when the AppImage is started through a symlink named `orb` or with `--orb` as the first argument |
| `OpenRB-338.desktop` (and `usr/share/applications/`) | `Name=OpenRB-338`, `Exec=OpenRB-338 %f`, `Categories=AudioVideo;Audio;Music;`, `StartupWMClass=OpenRB-338` (the app id the window sets, so Wayland and X11 shells match the window to the entry) |
| `OpenRB-338.png`, `.DirIcon`, `usr/share/icons/hicolor/{16,32,64,128,256,512}.../apps/OpenRB-338.png` | `assets/ui/app_icon.svg` rendered by `ui-art iconset` |
| `usr/bin/OpenRB-338`, `usr/bin/orb` | the two programs |
| `LICENSE`, `usr/share/doc/OpenRB-338/LICENSE` | GPL-3.0-only |

**The CLI is inside** (decision of this task, since nothing decided otherwise). A user who has
only the AppImage then also has the command line tool, and `orb` costs only a second binary of the
same code (the engine and the embedded standard kit are in both). Use it as
`OpenRB-338-<version>-x86_64.AppImage --orb render SONG.rbs -o SONG.flac`, or make a symlink
`ln -s OpenRB-338-<version>-x86_64.AppImage ~/bin/orb` and run `orb render ...`: the AppImage
runtime sets `ARGV0` to the name it was called by, and `AppRun` tests it. Measured on the
aarch64 AppImage in an `ubuntu:22.04` container, under `--appimage-extract-and-run` too: a
symlink named `orb` printed the same help as `--orb --help`. The `.tar.gz` stays for anyone who
wants a plain `orb` without the AppImage.

**What is bundled and why.** Only `OpenRB-338` and `orb`. No library: both binaries are Rust
with the C parts (the vendored libvorbis) linked in statically, and what remains is the system
set below, which must come from the host (a bundled `libasound`, GL or X11 would have to match
the host's sound server, driver and display stack, and bundling glibc or the GL stack is what
breaks AppImages on other distributions). `scripts/appimage-smoke.sh` checks that the AppDir has
exactly two ELF files and no `libc.so`, `ld-linux`, `libm`, `libpthread` or `libdl`.

**The host provides:**
- glibc 2.35 or newer (Ubuntu 22.04, Debian 12, Fedora 36 and newer; the most recent symbol the
  programs use is `GLIBC_2.35`, see "What may ship"; both architectures);
- `libasound.so.2` (`libasound2` on Debian and Ubuntu, `alsa-lib` on Fedora and Arch): without
  it even `orb --help` fails; a PipeWire or PulseAudio system has the ALSA plugins for it;
- for the window: X11 (`libxcursor1`, `libxrandr2`, `libxi6`, `libxkbcommon-x11-0`; the X11
  libraries are loaded at run time) or Wayland (`libwayland-client0`, `libxkbcommon0`), and
  a graphics driver, Vulkan or OpenGL, either one. The window is drawn by wgpu, whose backends
  are Vulkan and OpenGL (EGL) here (Metal and DX12 are on the other systems). egui-wgpu asks
  for all of `Backends::PRIMARY | GL` with `PowerPreference::HighPerformance`, and wgpu sorts
  every adapter it finds by device type (discrete GPU, integrated GPU, other, virtual, CPU;
  wgpu-core `get_order`), the first one wins, and a tie keeps the backend order, Vulkan before
  OpenGL. So OpenGL is used when no Vulkan adapter exists, and the `WGPU_BACKEND` variable
  (`vulkan` or `gl`) forces one. Vulkan needs
  `libvulkan1` and an ICD (Mesa: `mesa-vulkan-drivers`, which has lavapipe, software
  rendering). OpenGL needs **`libegl1`** (`libEGL.so.1`, which wgpu loads at run time) and a
  Mesa or vendor EGL driver (`libegl-mesa0`, `libgl1-mesa-dri`; a desktop has them). Without
  `libEGL.so.1` and without a Vulkan ICD the program stops with
  "FailedToCreateSurfaceForAnyBackend"; that was the error of the first task-112 smoke run,
  whose OpenGL run only lacked `libegl1` (task-135: the fallback needed no wgpu setting).
  `ORB_GPU_INFO=1` makes OpenRB-338 print the adapter it chose (`gpu: Vulkan Cpu
  llvmpipe ...`). The smoke test (xvfb, `ubuntu:22.04`, aarch64) takes a `--screenshot` with
  `WGPU_BACKEND=vulkan`, with `WGPU_BACKEND=gl`, with nothing forced and a Vulkan ICD present
  (it chose Vulkan: lavapipe and Mesa's OpenGL are both device type CPU, a tie, so the backend
  order decides), and with the Vulkan ICDs hidden (`VK_ICD_FILENAMES=/nonexistent`), which
  chose OpenGL; each asserts the backend. Not tried on a real desktop (task-134).

FUSE 2 (`libfuse2`) lets the AppImage mount itself. Without it, run
`./OpenRB-338-<version>-x86_64.AppImage --appimage-extract-and-run`, or `--appimage-extract` and
`squashfs-root/AppRun`.

**Checked (2026-10-29, this Mac, Docker Desktop on Apple silicon; the CLI was still called `rb`
then, task-068 renamed it `orb`).** Both AppImages were built
for real with `scripts/package-appimage.sh` (binaries from `scripts/build.sh` in the Docker
image, `SOURCE_DATE_EPOCH=1793215800`) and passed `scripts/appimage-smoke.sh`:

| AppImage | Size | sha256 |
|---|---|---|
| `OpenRB-338-2026.1028.1930-x86_64.AppImage` | 15317496 B | `088ec470...2827a1` |
| `OpenRB-338-2026.1028.1930-aarch64.AppImage` | 14547464 B | `def28e2c...178a9d` |

The smoke test found: the desktop entry valid (`desktop-file-validate`), the six icon sizes and
`LICENSE`; exactly two ELF files (`usr/bin/OpenRB-338`, `usr/bin/rb`) and no glibc file;
the highest glibc symbol `GLIBC_2.35` (OpenRB-338) and `GLIBC_2.34` (rb), both architectures
(`objdump -T`); `rb --help` and `rb info` on `tests/songs/shipped/Calm.rbs`; the symlink named
`rb`; and a PNG (191621 B on aarch64, 191815 B on x86_64) from `--demo --screenshot` under xvfb
through Vulkan (lavapipe). The aarch64 AppImage ran as an AppImage (`--appimage-extract-and-run`,
native on Apple silicon). **The x86_64 one could not be exec-ed in Docker Desktop's amd64
emulation** (`Exec format error` for the AppImage runtime and for the x86_64 appimagetool: the
ELF header has the AppImage marker in `e_ident`, ABI version 65), although the static x86_64
programs inside run. So, only when `ORB_APPIMAGE_EMULATED` is `1` (the scripts compute it from `uname`
for x86_64 on a macOS host and never inherit it; `release.yml` never sets it, and on Linux and in CI a runtime that does not run fails the
script), `scripts/appimage-pack.sh` unpacks appimagetool with `unsquashfs`, and the smoke test
unpacks the x86_64 AppImage's squashfs and runs its `AppRun` (with `ARGV0` set by hand for the
symlink test). This emulated run is local only: everything but the x86_64 runtime was
exercised, and the x86_64 runtime itself (and so `--screenshot` through it) was not. A real x86_64 Linux host or the CI runner runs it. The first release
run (task-134) still has to check the runners and a real desktop.

## What the system shows for an unsigned download

Nothing is signed or notarized (decided 2026-10-28). Expect:

- macOS (Gatekeeper): a file downloaded in a browser carries the quarantine attribute, and a
  command-line binary or an app from it is blocked ("cannot be opened because the developer
  cannot be verified"). Remove the attribute: `xattr -dr com.apple.quarantine <file or app>`.
  For an app: on macOS 15 (Sequoia) and newer, open it once (it is refused), then System
  Settings > Privacy & Security > Security > "Open Anyway" and confirm (Sequoia removed the
  Control-click > Open bypass for unnotarized apps); on macOS 14 and older, right-click > Open >
  Open once works. Binaries built on the Mac itself and `curl`ed files are not quarantined.
  Apple silicon also requires a signature to run any binary: the linker adds an ad-hoc one to
  what cargo builds, which is enough locally and in the archive. The quarantine of a downloaded
  `.dmg` applies to the app copied out of it (the `README.txt` in the image says the same).
  These Gatekeeper paths were not tried here (no download, and this Mac is not on 15 for a
  quarantined app); only the local launch from the mounted image was.
- Windows (SmartScreen): "Windows protected your PC", "Microsoft Defender SmartScreen prevented
  an unrecognized app from starting": the publisher is shown as "Unknown publisher" (an
  Authenticode certificate would name it). Click "More info", then "Run anyway". It applies to a
  file downloaded in a browser (it carries the Mark of the Web), to the installer, and again to
  `OpenRB-338.exe` and `orb.exe` from the `.zip`. The installer asks for no administrator rights,
  so there is no UAC prompt with "Unknown publisher" on top of it. The warning fades with
  SmartScreen's reputation for the file's hash, which a new build per version starts again.
  Right-click the file > Properties > Unblock (or `Unblock-File`) removes the mark.
  This was not tried here (no download on the test machine).
- Linux: nothing; `chmod +x` if the file lost its mode.
