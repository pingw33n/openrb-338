# Building and packaging

How rb (the CLI) and OpenRB-338 (the UI, crate `rb-ui`) are built for release on the six
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
| `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` | `.tar.gz` | built on Ubuntu 22.04: glibc 2.35 or newer |
| `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc` | `.zip` | MSVC, not gnu: it is what Rust, eframe's wgpu and the runners use; `cargo-xwin` cross-builds it from a Mac or Linux |
| `x86_64-apple-darwin`, `aarch64-apple-darwin` | `.tar.gz` | two packages, no universal binary (the `.dmg` has the `lipo` of the two, below) |

A package is `OpenRB-338-<YYYY_MM_DD_HH_MM_SS>-<target>.tar.gz|zip` holding a folder of the same
name with `rb` (`rb.exe`), `OpenRB-338` (`OpenRB-338.exe`), `LICENSE` and `README.txt`. The
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
`PROFILE` (`crates/rb-ui/build.rs` uses `OUT_DIR` and the version variables only), so the profile
changes no behavior.

Fidelity: a dist-built `rb` (`target/aarch64-apple-darwin/dist/rb compare --manifest
tests/refs/manifest.json --mod-dir DIR`, the mods from `rb mod recrypt --iso "$RB_CD_ISO" -o
DIR`) compares 175 of 175 corpus songs equal, as the release build does.

Sizes on this Mac (2026-10-28, bytes; release profile with `strip = symbols`, then dist):

| Target | Binary `rb` | Binary `OpenRB-338` | Package |
|---|---|---|---|
| aarch64-apple-darwin | 5,924,320 to 5,418,736 | 21,058,864 to 14,981,536 | .tar.gz 15,417,562 to 11,888,843 |
| x86_64-apple-darwin | 6,170,656 to 5,749,832 | 22,011,896 to 16,373,416 | .tar.gz 15,940,752 to 12,531,940 |
| universal-apple-darwin | 12,101,088 to 11,185,904 | 43,078,960 to 31,365,536 | .dmg 34,302,700 to 26,980,214 |
| aarch64-pc-windows-msvc | 5,666,816 to 5,509,120 | 21,507,072 to 17,240,576 | .zip 15,703,374 to 12,583,557 |
| x86_64-pc-windows-msvc | 6,229,504 to 6,064,128 | 24,596,480 to 20,065,792 | .zip 16,469,940 to 13,365,072 |

The Linux packages were not built (Docker was not running).

### The version is the build time

Decided by the user (2026-10-28): no package, file name, plist or text shows the `version` of
`Cargo.toml` (it stays, cargo needs one). The version of OpenRB-338 is its UTC build time, the
same moment the About box shows (`2026-10-28 19:30:00 (7bd8876)`, task-102), from
`SOURCE_DATE_EPOCH` (default now; `scripts/package.sh`, `package-dmg.sh` and `build-all.sh`
export it once, release.yml takes it from `prepare`):

| Where | Form | Example |
|---|---|---|
| file names, `scripts/version.sh` (also the form of the release tag, task-113) | `YYYY_MM_DD_HH_MM_SS` | `OpenRB-338-2026_10_28_19_30_00-universal-apple-darwin.dmg` |
| About box, `README.txt` | `YYYY-MM-DD HH:MM:SS` | `2026-10-28 19:30:00` |
| macOS `CFBundleShortVersionString` and `CFBundleVersion` (at most three integers) | `YYYY.MMDD.HHMMSS` | `2026.1028.193000` |
| macOS `OpenRBBuildTime` | `YYYY-MM-DD HH:MM:SS UTC` | `2026-10-28 19:30:00 UTC` |

`CFBundleVersion` is the time too, not a commit count: it orders builds across checkouts and
branches, and the .dmg's name carries it. Neither `rb` nor `OpenRB-338` has a `--version` flag,
so neither prints the Cargo version. Later packages (the Windows installers, the AppImages, the
draft release) name themselves the same way.

### What may ship

Decided by the user (2026-10-28): the built-in kit `assets/Standard.rbm` (the "Standard
ReBirth" drum samples), embedded in both binaries, may ship in the public release packages:
the `.tar.gz`, the `.zip`, the `.dmg`, and the later installer and AppImages.

On Linux, both `rb` and OpenRB-338 link `libasound.so.2` (ALSA, package `libasound2`), so even
`rb --help` fails without it. OpenRB-338 also loads X11 or Wayland and the GL or Vulkan driver at
run time (dlopen): on a bare Ubuntu 22.04 it needed `libxcursor1`, `libxrandr2`, `libxi6`,
`libxkbcommon-x11-0` and a driver (`mesa-vulkan-drivers` for software rendering). The most
recent glibc symbol the binaries use is `GLIBC_2.35` (OpenRB-338) and `GLIBC_2.34` (rb), on both
architectures, so the minimum is glibc 2.35. Both Linux packages were smoke-tested in
`ubuntu:22.04` containers (`rb --help`, `rb info` on a shipped song, and OpenRB-338
`--screenshot` under xvfb, which wrote a PNG), amd64 and arm64.

The Windows build of OpenRB-338 is still a console-subsystem exe (a console window opens
behind it); the GUI subsystem is part of task-111.

## Scripts (`scripts/`)

| Script | Does |
|---|---|
| `scripts/build.sh TARGET` | builds `rb` and `OpenRB-338` (profile `dist`) for the target into `dist/<target>/bin/` |
| `scripts/package.sh TARGET` | `build.sh`, then packs `dist/<target>/OpenRB-338-<YYYY_MM_DD_HH_MM_SS>-<target>.tar.gz|zip` (`SKIP_BUILD=1` packs the existing `bin/`) |
| `scripts/build-all.sh [TARGET...]` | packages every target practical from this machine, and lists the skipped and failed ones |
| `scripts/package-dmg.sh` | builds both macOS targets, then `dist/universal-apple-darwin/OpenRB-338-<YYYY_MM_DD_HH_MM_SS>-universal-apple-darwin.dmg` (`SKIP_BUILD=1` uses the existing `bin/` folders) |
| `scripts/macos-app.sh OUT_DIR UI [CLI]` | makes `OUT_DIR/OpenRB-338.app` from the binaries (Info.plist, icon, ad-hoc signature) |
| `scripts/macos-dev-app.sh` | wraps `target/release/OpenRB-338` into `target/OpenRB-338.app` for everyday runs |
| `scripts/version.sh` | prints the build time `YYYY_MM_DD_HH_MM_SS` (UTC, from `SOURCE_DATE_EPOCH`), the version of the packages |
| `scripts/release-env.sh` | prints the build environment all packages of a release share (`SOURCE_DATE_EPOCH`, `RB_BUILD_BRANCH`) |
| `scripts/docker/linux-build.Dockerfile` | the Linux build image (Ubuntu 22.04, build tools, ALSA headers) |

`dist/` is git-ignored. The later packaging scripts take `dist/<target>/bin/` as their input.

The version string (Help > About, task-102) is made by `crates/rb-ui/build.rs` from two
environment variables that the scripts pass through, including into Docker: `SOURCE_DATE_EPOCH`
(the build time; default now) and `RB_BUILD_BRANCH` (`main` makes the hash the main form; a
tag's detached HEAD needs it). Example: `SOURCE_DATE_EPOCH=1793181022 RB_BUILD_BRANCH=main
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
cargo install cargo-xwin --locked
rustup target add --toolchain 1.99.1 x86_64-apple-darwin x86_64-pc-windows-msvc aarch64-pc-windows-msvc
```

`build.sh` adds the target of the build itself (`rustup target add`), so the last line is only a
way to fetch them ahead. `clang-cl` must be on `PATH` (Homebrew's `/opt/homebrew/opt/llvm/bin`;
Swift's toolchain also provides one). Docker Desktop must be running for the Linux builds; the
scripts do not start it. The Windows C build sets `CL=-Wno-incompatible-pointer-types`: the
vendored libvorbis (`aotuv_lancer_vorbis_sys`) passes a `float *` to `_mm_prefetch`, which MSVC's
`cl` accepts with a warning and clang-cl 21 and newer rejects (cargo-xwin overwrites `CFLAGS_*`
and `CC_*`, so the variable `CL`, which clang-cl reads itself, is the way in).

### Docker (Linux builds off Linux)

`build.sh` builds the image `openrb-linux-build:<arch>` and runs itself inside it with the
checkout mounted at its own path (and the git common dir read-only, for a worktree's `.git`
file, so the version string has its hash). The Rust toolchain pinned in `rust-toolchain.toml`,
the cargo registry and the target dir are in the named volumes `openrb-rustup-<arch>`,
`openrb-cargo-<arch>` and `openrb-target-<arch>`, so a second run is incremental.
`docker volume rm` them to start clean.

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
    run `tools/*.py` with Python 3 (every runner has it): `RB_PYTHON` if set, else the first of
    `python3`, `python`, `py -3` on PATH that is Python 3 (`tests/common/python.rs`); none found
    fails the test with a message naming `RB_PYTHON`. The tests that need `RB_CD_ISO`,
    `RB_THIRDPARTY_MODS_DIR` or `RB_THIRDPARTY_SONGS_DIR` (the only external inputs of the
    tests; AGENTS.md) skip when those are absent (checked locally with all of them unset). The
    synthetic recrypt tests (`crates/rbm/tests/recrypt_python.rs`, which run `tools/rbm.py`)
    need none of them.
    Linux installs `libasound2-dev` first.
- `release.yml`, on a tag `v*` and by hand:
  - `prepare` computes one `SOURCE_DATE_EPOCH` and one `RB_BUILD_BRANCH` (`scripts/release-env.sh`)
    so all packages agree.
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
    (`OpenRB-338-<YYYY_MM_DD_HH_MM_SS>-universal-apple-darwin.dmg`). It reuses the two package
    builds and the `prepare` build time, so the app's version string is theirs.
  - The draft release with the changelog (task-113), the Windows installers (task-111) and the
    AppImages (task-112) are added to this workflow by those tasks.

Validation before merge: `actionlint .github/workflows/*.yml` and `shellcheck scripts/*.sh`
(both from Homebrew). `act` was not used: it cannot run the macOS and Windows runners.

## The macOS app and .dmg (task-110)

Double-clicking a bare binary in Finder opens Terminal, and the Dock and Activity Monitor show
the generic executable icon for it (the icon comes from the bundle's `CFBundleIconFile`; eframe
only sets the Dock icon at run time). The `.app` bundle fixes both.

`scripts/package-dmg.sh` (Mac only) builds
`dist/universal-apple-darwin/OpenRB-338-<YYYY_MM_DD_HH_MM_SS>-universal-apple-darwin.dmg`:

- `OpenRB-338.app`, made by `scripts/macos-app.sh`: `Contents/MacOS/OpenRB-338` is the UI as a
  universal binary (`lipo` of the aarch64 and x86_64 builds; `lipo -archs` says `x86_64 arm64`),
  `Contents/Resources/OpenRB-338.icns` is the `app_icon` part rendered at 16 to 1024 px
  (`cargo run -p rb-ui-art --bin ui-art -- iconset DIR`, DIR required, with the same resvg as
  the panel, then `iconutil`), and `Contents/Info.plist` has `CFBundleName` and
  `CFBundleDisplayName` OpenRB-338, the identifier `net.emphased.openrb338` (the user's choice,
  2026-10-28), `CFBundleShortVersionString` and `CFBundleVersion` = the build time as
  `YYYY.MMDD.HHMMSS` (see "The version is the build time"), `OpenRBBuildTime`,
  `LSMinimumSystemVersion` = the highest `minos` of the binary (`scripts/build.sh` sets
  `MACOSX_DEPLOYMENT_TARGET=11.0` for both macOS targets, so both slices need macOS 11.0;
  `macos-app.sh` fails if it finds no `minos`), `NSHighResolutionCapable` and
  `LSApplicationCategoryType` `public.app-category.music`.
- The CLI `rb` (later `orb`, task-068) is inside the bundle, `Contents/MacOS/rb`, also
  universal. Why inside and not next to the app: dragging the app to Applications installs both
  and nothing is left behind, the bundle's seal covers both, and an update replaces both.
  The cost is a longer path; `README.txt` in the image gives
  `sudo ln -s /Applications/OpenRB-338.app/Contents/MacOS/rb /usr/local/bin/rb`.
- An `Applications` link, `LICENSE` and `README.txt` (install, Gatekeeper, the `rb` link). There
  is no background picture or icon layout (that needs Finder scripting, which is not
  reproducible on a runner).
- Signing is ad hoc only (Apple silicon refuses an unsigned bundle; no Developer ID, no
  notarization): `rb` is signed explicitly (`codesign --force -s -`; before, only the linker had
  signed it on its own), then the bundle without `--deep`, whose seal covers both executables,
  then `codesign --verify --deep --strict`.
- Reproducibility: `cp -p` and `touch -t` from `SOURCE_DATE_EPOCH` fix the times of the bundle
  and the image's files. Two runs of `SKIP_BUILD=1 scripts/package-dmg.sh` give images whose
  mounted files are byte-identical (the same sha256 for every file), but the `.dmg` files
  themselves differ (`cmp`: the header; hdiutil writes a new volume UUID and creation date),
  and hdiutil has no option for a fixed one, so the image is not byte-reproducible.

`scripts/macos-dev-app.sh` is for everyday runs: after `cargo build --release -p rb-ui` it copies
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
- Windows (SmartScreen): "Windows protected your PC": More info > Run anyway.
- Linux: nothing; `chmod +x` if the file lost its mode.
