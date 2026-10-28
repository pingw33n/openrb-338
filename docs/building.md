# Building and packaging

How rb (the CLI) and OpenRB-338 (the UI, crate `rb-ui`) are built for release on the six
targets, locally and in GitHub Actions (task-108). The installers and bundles (.dmg, Windows
installer, AppImage) and the draft release build on this (tasks 110 to 113).

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
| `x86_64-apple-darwin`, `aarch64-apple-darwin` | `.tar.gz` | two packages, no universal binary (a `lipo` of the two is task-110's job for the .app) |

A package is `OpenRB-338-<version>-<target>.tar.gz|zip` holding a folder of the same name with
`rb` (`rb.exe`), `OpenRB-338` (`OpenRB-338.exe`), `LICENSE` and `README.txt`. The version is
the workspace version (`scripts/version.sh`). The binaries are built with `--locked` and
stripped (`CARGO_PROFILE_RELEASE_STRIP=symbols`; the profile in `Cargo.toml` is unchanged).

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
| `scripts/build.sh TARGET` | builds `rb` and `OpenRB-338` for the target into `dist/<target>/bin/` |
| `scripts/package.sh TARGET` | `build.sh`, then packs `dist/<target>/OpenRB-338-<version>-<target>.tar.gz|zip` (`SKIP_BUILD=1` packs the existing `bin/`) |
| `scripts/build-all.sh [TARGET...]` | packages every target practical from this machine, and lists the skipped and failed ones |
| `scripts/version.sh` | prints the workspace version |
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
    `cargo test --workspace --locked`. The tests that need `original/`, `RB_MOD_DIR`, `RB_CD_ISO`
    or `RB_THIRDPARTY_DIR` skip when those are absent (checked locally with all of them unset).
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
  - The draft release with the changelog (task-113), the macOS .dmg (task-110), the Windows
    installers (task-111) and the AppImages (task-112) are added to this workflow by those tasks.

Validation before merge: `actionlint .github/workflows/*.yml` and `shellcheck scripts/*.sh`
(both from Homebrew). `act` was not used: it cannot run the macOS and Windows runners.

## What the system shows for an unsigned download

Nothing is signed or notarized (decided 2026-10-28). Expect:

- macOS (Gatekeeper): a file downloaded in a browser carries the quarantine attribute, and a
  command-line binary or an app from it is blocked ("cannot be opened because the developer
  cannot be verified"). Remove the attribute: `xattr -d com.apple.quarantine <file>`; for an app,
  right-click > Open once. Binaries built on the Mac itself and `curl`ed files are not
  quarantined. Apple silicon also requires a signature to run any binary: the linker adds an
  ad-hoc one to what cargo builds, which is enough locally and in the archive.
- Windows (SmartScreen): "Windows protected your PC": More info > Run anyway.
- Linux: nothing; `chmod +x` if the file lost its mode.
