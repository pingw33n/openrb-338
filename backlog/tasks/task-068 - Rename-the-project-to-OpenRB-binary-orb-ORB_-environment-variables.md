---
id: TASK-068
title: 'Rename the project to OpenRB-338: binary orb, ORB_ environment variables'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-27 11:27'
updated_date: '2026-10-29 09:18'
labels:
  - cli
  - tools
  - docs
  - tests
milestone: m-4
dependencies: []
references:
  - AGENTS.md
priority: medium
type: chore
ordinal: 94000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requested by the user on 2026-10-27. Unassigned, not in a milestone.

Rename the project from rb to OpenRB-338 (the user's name, 2026-10-27; first written as "OpenRB"), with the binary `orb`, and every environment variable to the `ORB_` prefix.

Scope:
- Binary: crates/rb-cli's [[bin]] name "rb" becomes "orb"; every user-facing string that names the command (help, errors such as "run `rb recrypt ...`" and "`rb standard-mod --rebirth-dll ...`", the default Standard.rbm next to the binary, `rb play`'s messages) says `orb`.
- Crates: rb-cli, rb-engine, rb-player (and rb-ui when task-044 adds it) become openrb-* or orb-* (choose one, say why; directories, package names, `use` paths). The rbm and rbs crates keep their names: RBM and RBS are ReBirth's file formats, not the project's name. The same rule everywhere: "rb" meaning this project becomes OpenRB-338 (prose; the full name, as the original is ReBirth RB-338) / orb (command, crate prefix if chosen); RB-338, ReBirth, .rbs, .rbm, RB20FUL.DAT and the like are the original's names and stay.
- Environment variables: RB_RB20FUL, RB_MOD_DIR, RB_RBM_DIR (a deprecated alias: drop it or keep it as ORB_RBM_DIR; say which), RB_MOD_ORIG_DIR, RB_STANDARD_MOD, RB_THIRDPARTY_DIR, RB_CD_ISO (task-065), RB_CD_REBIRTH_EXE (task-067), and any other RB_ variable in crates/, tools/, tests, docs/ and the backlog docs become ORB_* (ORB_RB20FUL, ORB_MOD_DIR, ...). Decide whether the old names stay as deprecated aliases for a while (suggested: no, the project has no outside users); say which.
- Python tools (tools/, tools/rbhost/) that invoke the binary or read the variables; tools/rbhost and rbcorpus.py keep their names unless they name the project (say what you renamed).
- Docs and agent instructions: AGENTS.md (title "the rb repository", commands, crate list), backlog doc-001 (the agent brief, via the backlog CLI), .claude/agents/*.md, docs/*.md, tests/refs/MANIFEST.md (generated: change the generator tools/rbcorpus.py and regenerate), the Cargo workspace, README if any. Open backlog tasks (To Do) whose descriptions name `rb ...` commands or RB_ variables get updated through the backlog CLI; Done tasks, their notes and the git history stay as written (they are history).
- On the test machine: rb's working folder C:\re\rb and its paths in tools and docs may stay (they are the machine's layout, not the product name) or move to C:\re\orb; say which, and do not touch C:\ReBirth RB-338.
- Out of scope and for the user: renaming the GitHub repository (github.com/pingw33n/rb) and the local checkout folder (/Users/pingw33n/dev/rb). List what would follow from those (the git remote, the Claude memory folder name, absolute paths in docs) in the final summary, without doing it.

Run at a quiet point: the rename touches almost every file, so no other task branch should be open (rebase conflicts); check `git worktree list` and the queue first. After the rename: cargo build/test --release (with the mod env vars under their new names), clippy, fmt, the Python tests, `orb compare --manifest ...` 164/164 (175/175), and `git grep -nwE 'rb|RB_[A-Z]'` reviewed so every remaining hit is a deliberate keep (list the kept classes).
Crate prefix: "openrb-338-" is awkward; "orb-" (the binary's name) or "openrb-" are fine; say which. The window title of the UI (m-3) and the app's About text say "OpenRB-338".
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The binary is orb, the crates follow the chosen prefix, and every user-facing message names orb; rbm/rbs and the original's names are unchanged
- [x] #2 Every environment variable uses the ORB_ prefix in code, tests, tools and docs; whether old names remain as aliases is stated
- [x] #3 AGENTS.md, the agent brief, .claude/agents, docs and open tasks name the project OpenRB-338 and the command orb; Done tasks and history are untouched; a git grep review lists only deliberate keeps
- [x] #4 cargo test --release, clippy, fmt, the Python tests and orb compare (164/164, 175/175) pass; the final summary lists what renaming the GitHub repo and the local folder would change
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Step 1: directory moves crates/rb-X to crates/orb-X, package names, [[bin]] orb, use paths. Step 2: env vars to ORB_* (RB_RBM_DIR dropped, OPENRB_* to ORB_*). Step 3: user-facing strings, scripts, workflows, Python tools, MANIFEST generator, docs, AGENTS.md, .claude/agents, doc-001 and open tasks via CLI. Step 4: verify build, tests with and without env, orb-ui debug, clippy, fmt, python, shellcheck, actionlint, orb compare 175/175, xwin clippy, git grep review. Step 5: settings dir keeps its name (no migration); ask the user.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
task-104 already named the desktop UI OpenRB-338 (binary, window title, app name, app_id); the rest of the rename is still here.

The user (2026-10-28): the CLI binary is exactly `orb`, never `orb-cli` or any other suffix, whatever the crate that builds it is called (the crate may be orb-cli; its [[bin]] name is orb). The desktop app's binary is already OpenRB-338 (task-104) and stays so.

Branch: worktree-agent-a23d8b64bf2924b75 (rebased on main f2ea5f7)

Decisions (by the supervisor's prompt, no strong reason against found):
- Crate prefix orb-: crates/rb-{cli,engine,player,ui,ui-art,testkit} became crates/orb-{...} (git mv, package names, lib name orb_cli, use paths rb_x -> orb_x). The [[bin]] of orb-cli is named exactly `orb` (so `cargo run -p orb-cli`, target/release/orb, CARGO_BIN_EXE_orb); the UI binary stays OpenRB-338 (crate orb-ui). rbs and rbm keep their names (ReBirth's formats). Why orb-: it is the binary's name, short, and does not collide with the UI's OpenRB-338.
- No deprecated aliases: the RB_* and OPENRB_* names are not read any more. RB_RBM_DIR (the old alias of RB_MOD_DIR) is dropped.
- C:\re\rb on the test machine stays (the machine's layout); C:\ReBirth RB-338 untouched. tools/rb_export.py, rbcmp.py, rbcorpus.py, rbhost, rb1x.py, rbm.py, rbs.py keep their names (they name ReBirth, RBS/RBM formats or the test-machine runners, and tests/refs/manifest.json records "tools/rb_export.py" as history).
- Settings folder: the UI still reads rb/ui.toml in the configuration folder (~/Library/Application Support/rb/ui.toml on macOS, %APPDATA%\rb on Windows, ~/.config/rb on Linux). I did not move it: a move would lose the user's saved settings without a migration (task-107: no migration) and I must not touch the real settings. Question for the user: rename the folder to OpenRB-338 (or orb)? Either needs a migration or a one-time manual move.
- Bundle id net.emphased.openrb338, the app name OpenRB-338, the installer name openrb.nsi, the Docker image and volume names openrb-linux-build / openrb-{cargo,rustup,target}-<arch>, the repo URL github.com/pingw33n/open-rb-338 (About box) stay.

Environment variables (new name <- old name; docs/building.md "Environment variables" has the table):
| New | Old | Read by |
| ORB_CD_ISO | RB_CD_ISO | orb-testkit (gated tests) |
| ORB_THIRDPARTY_MODS_DIR | RB_THIRDPARTY_MODS_DIR | orb-testkit |
| ORB_THIRDPARTY_SONGS_DIR | RB_THIRDPARTY_SONGS_DIR | orb-testkit |
| ORB_MOD_DIR | RB_MOD_DIR | orb render/compare/play/info, OpenRB-338 |
| ORB_RB20FUL | RB_RB20FUL | orb mod recrypt, tools/rbcorpus.py |
| ORB_UI_CONFIG | RB_UI_CONFIG | OpenRB-338 settings file |
| ORB_UI_VERSION | RB_UI_VERSION | build.rs -> env! (never set by hand) |
| ORB_BUILD_BRANCH | RB_BUILD_BRANCH | build.rs, scripts/release-env.sh, build.sh, release.yml |
| ORB_GPU_INFO | OPENRB_GPU_INFO | OpenRB-338, appimage-smoke.sh |
| ORB_APPIMAGE_EMULATED | OPENRB_APPIMAGE_EMULATED | package-appimage.sh, appimage-pack.sh, appimage-smoke.sh |
| ORB_PYTHON | RB_PYTHON | tests/common/python.rs |
| ORB_LIVE_OUT | RB_LIVE_OUT | orb-player live_messages test |
| ORB_REBIRTH_DLL | RB_REBIRTH_DLL | tools/gen_static_tables.py |
Removed, not renamed (git grep at the base commit finds no reader): RB_RBM_DIR (alias, dropped; its read in mods.rs and its env_remove lines in four test files are gone), RB_STANDARD_MOD (the lookup went with task-067; only crates/orb-cli/tests/standard_mod.rs still set it to check it is ignored: that check and the env parameter of its render() helper are removed), RB_MOD_ORIG_DIR, RB_CD_REBIRTH_EXE (only named in a docs/test_machine.md sentence), RB_THIRDPARTY_DIR (only in old task text). Not renamed: RBM_TEST_DAT / RBM_TEST_MODS (set by a cargo test for tools/rbm_test.py only; RBM is the mod format), SOURCE_DATE_EPOCH, GITHUB_REF_NAME, WGPU_BACKEND (other tools').

Other renames: user-facing prefix "rb: " of the CLI's messages -> "orb: " (warnings, errors, progress), clap name "orb", engine/compare.rs describe() prints "orb" instead of "rb" (a test in compare.rs updated), orb-player's "rb-player: audio output" -> "orb-player:", the browser's "needs orb mod recrypt" and the error hints "orb mod recrypt", mod.toml's header comment "# orb mod unpack", the egui widget ids "rb_*" -> "orb_*", test helpers fn rb() -> fn orb(), tools/thirdparty_mods_manifest.py --rb -> --orb (find_orb), tools/rbhost/hashcmp.py --rb -> --orb and its JSON key rb_sha256 -> orb_sha256, temp folders rb-testkit-/rb-ui-startup- -> orb-. Packaging: orb / orb.exe / Contents/MacOS/orb (dmg, sign step, README hint with the `ln -s .../orb /usr/local/bin/orb`), the AppImage AppRun routes a symlink named `orb` or `--orb` first argument to usr/bin/orb, appimage-smoke.sh uses links/orb and --orb, the NSIS installer ships orb.exe with the component "Add orb to the PATH of the current user", README.txt/package.sh texts. Commit-subject areas in AGENTS.md (engine, cli, tools, tests, docs, backlog) are unchanged.

Deliberate keeps (what the final `git grep -nwE 'rb|RB_[A-Z]'` still finds, besides the old-name table in docs/building.md):
- [SUPERSEDED 2026-10-29: the settings folder is now OpenRB-338, see the review-round notes] the settings folder name rb (rb/ui.toml, %APPDATA%\rb) in crates/orb-ui/src/settings.rs, tests/settings.rs, main.rs doc, docs/ui_spec.md, docs/building.md, scripts/package-installer.sh;
- C:\re\rb and rcs path rb/ (docs/test_machine.md, docs/building.md, tools/rbhost/*, tools/rb_export.py, doc-001);
- tools/rb_export.py, rb1x.py, rbcmp.py, rbm.py, rbs.py, rbcorpus.py file names and their mentions; manifest.json "tool" strings (committed reference data, history);
- Python/C file modes "rb" (open(path, "rb"), rb"..." literals, fopen);
- "rb-recrypt.pad" (PAD_NAME, a chunk name written into every recrypted mod and pinned by SHA-256) and "rb mod pack-standard" inside the description of assets/Standard.rbm (INFO_TEXT in crates/rbm/src/standard.rs; changing it changes the committed asset's bytes; a comment says so; docs/rbm.md says so);
- the Ogg stream serial 0x7262 ("rb") in crates/orb-cli/src/encode.rs (output bytes);
- ~/rb-mods, ~/rb-songs, ~/rb_stuff (the user's folders), github.com/pingw33n/rb and open-rb-338 URLs;
- RB-338, ReBirth, .rbs, .rbm, RB20FUL.DAT, rbhost, rbs/rbm crates, RBM_TEST_*;
- dated "Checked" records in docs/building.md that ran the CLI as rb/rb.exe (a sentence at the top of each says so), RB_STANDARD_MOD and `rb standard-mod` named as the history of task-036/067 in docs/rbm.md;
- Done tasks, their notes, git history (untouched).

Backlog changes: doc-001 (the brief; 4 sentences) via `backlog doc update`. Open tasks: task-117 (unassigned) and task-129 (@agent-complex) updated with the CLI (description, references, task-129's ACs: crates/orb-*, ORB_ variables, orb render/compare). [Superseded 2026-10-29: 114, 115 and 134 were updated with the user's permission, task-037 and draft-001 in round 1] Not touched: task-114, task-115, task-134 (assigned to @pingw33n: they name crates/rb-ui-art, crates/rb-ui, RB_BUILD_BRANCH, OPENRB_GPU_INFO, a symlink named rb), task-037 (its only hits are the supervisor notes about the history of `rb standard-rbm` / `rb standard-mod` of task-036, and its goal is done by task-067), titles of all tasks ("rb-ui: ..." prefixes are history).

Method: crates moved with git mv; every content change was made with the Edit tool (no sed or python). Leftover check script: grep for RB_/OPENRB_/rb-crate/rb_crate tokens outside the keeps above, empty at the end.

Failed first attempt (fixed): replacing the text in INFO_TEXT made 4 tests fail (the_embedded_mod_is_a_standard_mod, the_embedded_kit_is_the_standard_mod, the_command_reproduces_the_committed_mod, pack_standard_from_the_image_equals_the_committed_mod): the string is baked into assets/Standard.rbm. Reverted; see the keep above.

Verification (all in this worktree):
- cargo build --release: ok.
- cargo test --release -q --no-fail-fast, with ORB_CD_ISO="/Users/pingw33n/Downloads/Rebirth 2.01 (2001 version).iso" ORB_THIRDPARTY_MODS_DIR=~/rb-mods/mods ORB_THIRDPARTY_SONGS_DIR=~/rb-songs/songs: 586 passed, 0 failed, 3 ignored. Without the variables: 586 passed, 0 failed, 3 ignored (gated tests skip).
- cargo test -p orb-ui (debug): 197 passed, 0 failed, 2 ignored.
- cargo clippy --all-targets -- -D warnings: clean. cargo +nightly fmt --all --check: clean.
- Python: unittest discover -s tools: 66 tests OK (3 skipped); -s tools/rbhost: 43 tests OK.
- shellcheck scripts/*.sh and actionlint .github/workflows/*.yml: no output (clean).
- orb mod recrypt --iso <ISO> -o DIR (4 mods) then orb compare --manifest tests/refs/manifest.json --mod-dir DIR: 175 of 175 equal (exit 0). Two expected PCF wave-127 warnings print with the new "orb:" prefix.
- CL=-Wno-incompatible-pointer-types XWIN_ACCEPT_LICENSE=1 cargo xwin clippy --release -p orb-ui --target x86_64-pc-windows-msvc -- -D warnings: finished clean (build.rs, embed-resource included).
- tools/rbcorpus.py manifest regenerated tests/refs/MANIFEST.md (one line: `orb compare`, `orb mod recrypt`, $ORB_MOD_DIR, $ORB_RB20FUL).
- Not run: the dist build and packaging scripts (build.sh / package-*.sh need Docker and the installers' tools), the NSIS installer build, the AppImage smoke test. The edits there are names only (shellcheck clean); the first release run (task-134) will exercise them. A previously installed rb.exe in an install folder would stay behind after an upgrade with the new installer (the installer deletes only orb.exe); no outside users.

Review-round decisions of the user (2026-10-29): (1) the UI settings folder is now OpenRB-338 (OpenRB-338/ui.toml under the configuration folder: ~/Library/Application Support, %APPDATA%, XDG_CONFIG_HOME or ~/.config), no migration; settings of the old rb/ folder are not carried over. Changed crates/orb-ui/src/settings.rs, its doc, main.rs doc, tests/settings.rs (temp dirs only), docs/ui_spec.md, docs/building.md, scripts/package-installer.sh readme text; the real ~/Library/Application Support/rb/ was never read, moved or deleted. (2) task-114, task-115 and task-134 (assigned to @pingw33n, changed with the user's permission) got only the renamed identifiers: crates/rb-ui* -> crates/orb-ui*, -p rb-ui-art, RB_BUILD_BRANCH, OPENRB_GPU_INFO, OPENRB_APPIMAGE_EMULATED, the symlink named rb -> orb (descriptions, notes, task-134's AC #3, references). Stale rb.exe: an install folder of an older installer keeps its rb.exe after an upgrade with the new installer (it deletes only orb.exe); accepted, no outside users.

Review round 1 (of cae3edf): task-037 is To Do, not history: its description line 1 and 7, AC #1 and notes lines 5 and 7 (rb renders, rb standard-rbm, rb standard-mod) now say orb / orb mod pack-standard, through the CLI; backlog/drafts/draft-001 (crates/rb-engine -> crates/orb-engine, references and description) through backlog draft edit. The earlier note that task-037 was left as history is withdrawn. The About URL github.com/pingw33n/open-rb-338 stays: it is the intended future repository name (task-102, task-108) and is a dead link until the repo is renamed. Round 2 inputs of the user (settings folder OpenRB-338, tasks 114/115/134 updated) are recorded above. After both: debug cargo test -p orb-ui 197 passed, release workspace tests 586 passed with the gated inputs set, clippy and nightly fmt clean.
Review round 2: the commit body and the final summary now say the settings folder is OpenRB-338 (user decision 2026-10-29) and that tasks 114, 115, 134 were updated with permission; the superseded lines above are marked.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
OpenRB-338 rename done: the CLI binary is `orb` (crate orb-cli), the crates are orb-engine, orb-player, orb-ui, orb-ui-art, orb-testkit (rbs and rbm keep their names), and every environment variable is ORB_* (13 renamed, 5 removed because nothing reads them, no deprecated aliases; the table is in docs/building.md and in the notes). The UI binary OpenRB-338, the bundle id and the app name are unchanged.

Changed: crates (directories, package and lib names, use paths, user-facing messages "orb: ...", clap name), tests, scripts (orb in the tar/zip, the dmg's Contents/MacOS/orb and its `ln -s` hint, the AppImage AppRun route for a symlink named orb or --orb, appimage-smoke.sh, the NSIS installer's orb.exe and the "Add orb to the PATH" component), release.yml and ci.yml, Python tools (--orb options, ORB_ variables), tools/rbcorpus.py and the regenerated tests/refs/MANIFEST.md, AGENTS.md, doc-001, .claude/agents, docs/*.md, assets READMEs, and the open tasks task-117 and task-129 through the backlog CLI. Done tasks and history are untouched.

The UI settings folder is OpenRB-338 (OpenRB-338/ui.toml under the configuration folder), by the user's decision of 2026-10-29, with no migration: settings of the old rb/ folder are not carried over and the real one was never touched. Kept on purpose: C:\re\rb on the test machine, the file names of tools/rb_export.py, rbcmp.py, rbcorpus.py, rbhost, the chunk name rb-recrypt.pad and the description text inside assets/Standard.rbm (pinned bytes), the Ogg stream serial 0x7262, the user's ~/rb-mods ~/rb-songs ~/rb_stuff folders, the GitHub URLs, RBM_TEST_DAT/RBM_TEST_MODS, the Docker image and volume names openrb-*, openrb.nsi, and the dated "Checked" records in docs/building.md (marked as run under the old name). Task-114, task-115 and task-134 (assigned to @pingw33n) got only the renamed identifiers (crates/orb-ui*, ORB_BUILD_BRANCH, ORB_GPU_INFO, ORB_APPIMAGE_EMULATED, the orb symlink), with the user's permission; task-037 and draft-001 got the same renames in review round 1.

Numbers: cargo test --release --no-fail-fast 586 passed, 0 failed with ORB_CD_ISO, ORB_THIRDPARTY_MODS_DIR and ORB_THIRDPARTY_SONGS_DIR set and 586 passed, 0 failed without them; debug cargo test -p orb-ui 197 passed; clippy and nightly fmt clean; Python tests 66 + 43 OK; shellcheck and actionlint clean; orb compare --manifest tests/refs/manifest.json --mod-dir DIR (DIR from orb mod recrypt --iso) 175 of 175 equal; cargo xwin clippy --release -p orb-ui --target x86_64-pc-windows-msvc -- -D warnings clean. Not run: the dist build, the installer, dmg and AppImage packaging, the AppImage smoke test (names only, shellcheck clean; task-134 runs them).

If the user renames the GitHub repository (github.com/pingw33n/rb) and the checkout folder (/Users/pingw33n/dev/rb), these follow: `git remote set-url origin` (GitHub redirects the old URL but the remote should be updated); AGENTS.md line 4 (the sentence that the repository and folder are still called rb) and the origin URL in the Task ownership rule; the repository URL the About box and ui_spec.md link to (https://github.com/pingw33n/open-rb-338, a different name already, so it needs a decision of its own); Claude Code's memory and project folder for the checkout (~/.claude/projects/-Users-pingw33n-dev-rb, derived from the path) and the supervisor's MEMORY.md; the paths of the git worktrees under .claude/worktrees (their .git files and .git/worktrees/*/gitdir hold absolute paths: run `git worktree repair`); the local folders of the user's data that are named rb (~/rb-mods, ~/rb-songs, ~/rb_stuff) and every command line that names them (docs/building.md, crates/orb-cli/tests/thirdparty.rs); the GitHub Actions need no change (no workflow names the repository); the settings folder rb/ui.toml would be a separate decision.
Round 1 corrections: the About URL github.com/pingw33n/open-rb-338 is the intended future repo name and a dead link until the repo is renamed. The UI settings folder is OpenRB-338 (the earlier text saying it stays rb is superseded).
<!-- SECTION:FINAL_SUMMARY:END -->
