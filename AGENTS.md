# AGENTS.md: the rb repository

rb is a reimplementation in Rust of the audio engine of Propellerhead ReBirth RB-338 2.01. It renders an `.rbs` song, optionally with an `.rbm` mod, to audio. The rendered audio must match the original's "Export Loop as Audio File" output sample for sample, within a tolerance set by a `Decide:` task. A UI with a similar look and feel comes after the engine; it does not have to match the original exactly. MIDI and ReWire are out of scope for now.

The original program is in `original/`, and its IDA databases are in `ida/`. What is known about the engine is in `docs/initial_investigation.md`, and the official song format is in `docs/RBS42.txt`.

Planning, progress and decisions live in `backlog/` (Backlog.md). The shared agent brief covers the worktree, IDA, the Windows test machine and its permits, testing and rebasing. It is the backlog doc `agent-brief`: run `backlog doc view doc-001 --plain`, or read its file under `backlog/docs/`.

## Where things are

**Backlog access.** Tasks, documents and milestones live in `backlog/` and are read and changed only through Backlog.md:
- its MCP tools (the `backlog` MCP server) when they are loaded in the session;
- otherwise the `backlog` CLI, which truncates long documents in `doc view` and breaks on shell quoting.

**Useful commands.**

| Command | Shows |
|---|---|
| `backlog task list --ready --sort ordinal --plain` | The queue |
| `backlog task list -l decision -a @pingw33n --plain` | The pending decisions |
| `backlog milestone list --plain` | The plan |

**Backlog conventions.**
- A decision is a task labeled `decision` and titled `Decide: <title>`.
- What runs is the supervisor's agents. `In Progress` is never committed: a task goes from `To Do` to `Done` in its one commit.
- A milestone's title starts with its id (`m-1: ...`), because the web UI shows titles only.

**Milestones.**
- `m-0`: deterministic reference renders from the original, and the tools around them.
- `m-1`: the Rust engine and the `rb` CLI (`.rbs` to FLAC, Ogg or WAV).
- `m-2`: RBM mods.
- `m-3`: the desktop UI (egui, scalable SVG panel): play and edit songs live.

**Knowledge documents.** These are files edited like source: `docs/*.md`.
- `docs/initial_investigation.md` is the engine survey; its [C] items are confirmed and its [G] items are guessed.
- `docs/RBS42.txt` is the file format.
- Later tasks add `test_machine.md`, `song_load.md`, `export_state.md` and `rbm.md`.
- `docs/original_bugs.md` lists the defects of the original (what, addresses, trigger, effect, corpus songs, rb's model, [C]/[G]). A newly found defect of the original goes into `docs/original_bugs.md` as well as its home document.
- A new fact about the original goes into the document it belongs to, with addresses, in the same commit as the work.

**Assignees.**
- `@pingw33n` is the user and owns every decision.
- `@supervisor` is the orchestrator.
- The agent types, defined in `.claude/agents/`, are `@agent-complex` (opus), `@agent-worker` (sonnet) and `@agent-chore` (haiku). An agent writes its branch name in the task's notes.
- `agent-reviewer` (haiku, read-only) reviews branches and is never an assignee.

## Backlog rules

- **How the backlog is changed.** Only through Backlog.md's MCP tools or its CLI; never edit a file under `backlog/` by hand. Read `backlog instructions task-execution` before starting a task and `backlog instructions task-finalization` before finishing one. A multi-line text goes in with a real newline, never a literal `\n`.
- **Who is spawned on what.** The supervisor starts an agent only for a task assigned to an agent type. It assigns freely among unassigned tasks and tasks assigned to an agent type:
  - to `@agent-complex`: reverse engineering, the engine's DSP, sequencer and numerics, the offline DLL host, and experiments that need judgment;
  - to `@agent-worker`: bounded work such as the Python tools, file-format crates, the CLI, sample loading, test songs, corpus capture and docs;
  - to `@agent-chore`: mechanical work whose text or script is already written.

  A task assigned to `@pingw33n` is the user's: never change, reassign or spawn it unless the user asks.
- **Which task next.** Only tasks in a milestone count: the lowest open milestone first, then priority and ordinal. A task in no milestone is tracking or an idea.
- **Starting.**
  - The supervisor spawns the agent on the task as it is (`To Do`); there is no status change and no commit on main.
  - The agent works in its worktree with the `backlog` CLI there, because the MCP tools write to the main checkout.
  - The agent adds its branch name with `--append-notes`, researches, then records its plan with `--plan` before changing code.
  - Every agent is spawned in the background: the supervisor never blocks on one, keeps working the queue, and picks each agent up when its report arrives.
- **Working.** `--append-notes` holds, written as the work goes:
  - the evidence (addresses, functions, scripts run);
  - the numbers before and after;
  - the failed hypotheses;
  - the commands that reproduce a result.
- **Finishing.**
  - Verify each acceptance criterion with evidence and mark it with `--check-ac`.
  - Write `--final-summary` with the outcome, the branch and commits, the numbers and the tests run.
  - Then set `-s Done`, in the same commit as the work.
  - One commit per task. The branch holds a single commit, and review fixes amend it. It lands on main as that one commit (`git merge --squash`, or a fast-forward of the rebased commit), with the supervisor's merge comment in it. There are no separate start, merge or comment commits, and there is no Review status.
- **Review and merge.**
  1. The supervisor spawns the reviewer agent on the branch. The reviewer checks each acceptance criterion against the evidence and the standing rules, then returns a verdict (merge / send back) naming the specific defects, each with file:line.
  2. A task is merged only after a `merge` verdict, and only then does the supervisor run the tests and merge.
  3. On a `send back` verdict, the supervisor resumes the task agent's existing session (by its session id) with the defects. The agent records the review round in the task's notes and amends its one commit on the same branch; the supervisor never spawns a fresh task agent for a review round.
  4. Re-review resumes the same reviewer session (by its session id). Only the first review of a task spawns the reviewer fresh.

  Every task gets this reviewer pass before it is merged; the supervisor does not self-review and merge.
- **What you learn goes where it belongs.**
  - A fact about the original goes into `docs/`.
  - Follow-up work becomes a new task with labels, milestone, `--dep` and a description a future agent can act on alone.
  - A decision the task does not settle becomes a `Decide:` task (`--type spike -l decision,<area> -a @pingw33n`), with the context, the options and the recommendation in its description. Your task depends on it; do not make the decision yourself.
  - When the user decides, the `Decide` task gets a final summary with the outcome and the date, and `-s Done`.
  - Nothing is reported only in chat: your final chat message is the task id plus what the supervisor must do.
- **How a task is written.** It must be doable by an agent without this conversation. It contains:
  - why it exists, and the context not recoverable from the code;
  - testable acceptance criteria;
  - `--ref`;
  - labels, from: `re`, `engine`, `cli`, `tools`, `tests`, `testmachine`, `rbm`, `ui`, `docs`, `decision`;
  - a type (`bug`, `feature`, `enhancement`, `task`, `chore`, `docs`, `spike`);
  - a priority, a milestone and its dependencies.

  `--ordinal` is the queue order. `Draft` holds ideas.
- **Task ownership.** While a task is assigned to an agent, only that agent edits its file. The supervisor adds the merge comment in the task's squashed commit, pushes main to `origin` (github.com/pingw33n/rb, private) after each merge, and runs `backlog doctor` after the push. Agents never push.
- **Standing rules.**
  - Never edit `original/`, `ida/` or a committed reference render by hand.
  - Facts about the original carry addresses and are checked against the disassembly, not only Hex-Rays.
  - Engine code computes in f64, rounds to f32 exactly where the original stores a float, uses no FMA, and keeps the original's operand order.
  - An engine change is checked against the reference corpus.
  - Edit source with Edit/Write, not python or sed.
  - Spell "color" the American way.
  - Lines of Rust are at most 100 columns, enforced by nightly rustfmt (`rustfmt.toml`). Long string literals are split by hand (`\` continuation or `concat!`); rustfmt does not wrap them.

## Commit messages

Subject: `<area>: <what>`, at most 72 characters. The area is one of `engine`, `cli`, `tools`, `tests`, `docs` or `backlog`.

Body, in this order:
1. **Change**: what and where.
2. **Result**: the numbers before and after, what works now, and the corpus result for an engine change.
3. **Testing**: `cargo test N pass`, the corpus comparison, the tool run, or none.
4. `Task: task-NNN`, plus `Decision: task-NNN` when a decision was applied.
5. The attribution line the harness gives.

Everything else goes into the task's notes or a knowledge document.

## Build and test

The Rust workspace is at the repository root (`crates/rbs`, `crates/rb-engine`, `crates/rb-cli`, whose binary is `rb`). The toolchain is pinned in `rust-toolchain.toml`.

- Build: `cargo build --release`
- Test: `cargo test`. Tests that run `tools/*.py` find Python 3 through `tests/common/python.rs`: `RB_PYTHON` (a program path, or a command split on whitespace such as `py -3`) if set, else `python3`, `python` or `py -3` on PATH; none is a failure naming `RB_PYTHON`, not a skip.
- Lint: `cargo clippy --all-targets -- -D warnings`
- Format: `cargo +nightly fmt --all`; check: `cargo +nightly fmt --all --check`. `rustfmt.toml` uses unstable options (the 100-column limit, comment wrapping), so formatting needs nightly rustfmt. CI pins `nightly-2026-10-18` (`rustup toolchain install nightly-2026-10-18 --profile minimal --component rustfmt`, then `cargo +nightly-2026-10-18 fmt --all --check`); any recent nightly with rustfmt gives the same result locally. Plain `cargo fmt` (stable) still formats, but does not enforce the limit and warns about the unstable options. Only formatting uses nightly; the build and clippy use the pinned stable toolchain.
- Packages: `scripts/package.sh TARGET` builds `rb` and `OpenRB-338` for one of the six targets (x86_64/aarch64 for `unknown-linux-gnu`, `pc-windows-msvc`, `apple-darwin`) into `dist/<target>/` as `.tar.gz` (`.zip` on Windows); `scripts/build-all.sh` does all the targets this machine can build and lists the skipped ones. Linux targets need a running Docker off Linux, Windows targets need `cargo-xwin` off Windows. CI: `.github/workflows/ci.yml` (nightly fmt, clippy, tests on Linux, Windows and macOS) and `release.yml` (the six packages on native runners, on a `v*` tag or by hand). No signing (the macOS app is ad-hoc signed only). `scripts/package-dmg.sh` makes the universal `OpenRB-338-<YYYY_MM_DD_HH_MM_SS>-universal-apple-darwin.dmg` (package names carry the UTC build time, not the Cargo version; docs/building.md) (`scripts/macos-dev-app.sh` wraps the dev build into `target/OpenRB-338.app`). docs/building.md has the tools to install, the limits and the environment (`SOURCE_DATE_EPOCH`, `RB_BUILD_BRANCH`).
- Run: `cargo run --release -p rb-cli -- <args>`
- Golden test: `cargo test --release -p rb-engine --test corpus_golden` renders every corpus entry (diag, app at its fitted N, shipped) and prints each failing song's first difference. A debug `cargo test` skips the shipped songs (24 s in debug).
- Mods: rb loads only recrypted mods (`rb mod recrypt --rb20ful RB20FUL.DAT IN.rbm -o OUT.rbm`, the one command that reads `RB20FUL.DAT`; docs/rbm.md 12.4). `render`, `compare`, `play` and the mod tests take a folder of recrypted mods (`--mod-dir` / `RB_MOD_DIR`) and no `--rb20ful`.
- The built-in kit: rb embeds the standard mod `assets/Standard.rbm` ("Standard ReBirth",
  the 46 drum samples of ReBirth.dll and the panel of the CD's Rebirth.exe; assets/README.md),
  so `render`, `compare`, `play` and OpenRB-338 (the UI) need no kit option, variable or file
  (`--standard-mod` and `RB_STANDARD_MOD` are gone). `rb mod pack-standard --rebirth-dll
  original/ReBirth.dll --rebirth-exe CD-Rebirth.exe -o assets/Standard.rbm` rebuilds it byte for
  byte (the one command that reads ReBirth.dll and Rebirth.exe; `--iso` reads them from the CD
  image, and the gated test takes them from it; docs/rbm.md 13).
- External test inputs: the tests read exactly three variables, and every test that needs one
  skips with a message when it is not set (CI sets none). `RB_CD_ISO` is the ReBirth 2.01 CD
  image: the crate `rb-testkit` takes from it, once per test process, the DAT, `ReBirth.dll`,
  `Rebirth.exe` and the four shipped mods (each checked against its pinned SHA-256) and the four
  mods recrypted (pinned too), so no test needs `original/`, `RB20FUL.DAT` or a mod folder.
  `RB_THIRDPARTY_MODS_DIR` is the user's folder of RECRYPTED third-party mods and
  `RB_THIRDPARTY_SONGS_DIR` the user's folder of songs, both read in place (docs/rbm.md 11):
  `cargo test --release -p rb-cli --test thirdparty -- --nocapture`. The mods test checks the
  folder against the committed hash manifest `tests/refs/thirdparty_mods.json` and fails on a
  `.rbm` the manifest does not list, so regenerate it when the collection changes with
  `tools/thirdparty_mods_manifest.py --orig MODS.ORIG --mods MODS --dat RB20FUL.DAT` (it first
  recrypts the originals with the current rb and requires them to equal MODS byte for byte; the
  originals and the DAT are arguments, not variables). To re-verify the recryptor on the real
  collection, run `tools/thirdparty_mods_manifest.py --check --orig MODS.ORIG --dat RB20FUL.DAT`
  (recrypts the originals and compares with the manifest; exit 1 on a difference). Not test
  inputs: `RB_LIVE_OUT` (rb-player's live_messages: an output folder) and `RBM_TEST_DAT` /
  `RBM_TEST_MODS` (set by a cargo test for its child process only, `tools/rbm_test.py`).
- Song metadata: `rb info SONG.rbs [--mod-dir DIR] [--json]` prints the facts File > Info shows (format and version, title, text, web page, copyright, the mod and whether it is the built-in kit, a file of `--mod-dir` / `RB_MOD_DIR`, missing, or not checked, tempo, shuffle, mode, loop, synth sound) for an RBS 4.x or a ReBirth 1.x song; `--json` keys are in `rb info --help`. The code is `crates/rb-cli/src/info.rs`, shared with rb-ui.
- Corpus summary: `cargo run --release -p rb-cli -- compare --manifest tests/refs/manifest.json [--mod-dir DIR] [--set diag,app,shipped] [NAME...]` (the 11 mod songs need `--mod-dir` with the four recrypted shipped mods; `rb mod recrypt --iso "$RB_CD_ISO" -o DIR` makes them from the CD image) prints per song `EQUAL` or the first difference (frame, time, bar, channel, both values) and exits 1 on any difference. One song: `rb compare SONG.rbs --reference REF.flac|REF.wav [--idle-blocks N]`.
