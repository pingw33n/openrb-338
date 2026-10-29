---
id: doc-001
title: agent-brief
type: guide
created_date: '2026-10-26 09:03'
updated_date: '2026-10-29 07:51'
---
The shared brief for agents working on rb, the Rust reimplementation of the ReBirth RB-338 2.01 audio engine. Read in this order:
1. The root `AGENTS.md` (the backlog and commit rules).
2. This brief.
3. The task (`backlog task view task-N --plain`).
4. The documents the task points to.

The engine's facts, as reverse engineered so far, are in `docs/initial_investigation.md`. Read only the sections your task names (grep for the heading, then read that line range).

## Your worktree

You work in your own git worktree on your own branch, and everything is relative to its root. Do not touch the user's checkout.

**Committing.**
- Commit on your branch with the attribution line the harness gives and the root `AGENTS.md`'s message form.
- One commit holds the whole task: squash your work commits into it before you report, and amend it for review fixes.
- The task goes from `To Do` to `Done` in that commit. Never commit it `In Progress`.
- Do not merge or push, and rebase onto nothing but main.

**The backlog.** Change the task with the `backlog` CLI in your worktree, because the backlog MCP tools write to the user's checkout.

**Temporary files.** They go in a subdirectory of the session's scratchpad named after your branch. Never run a script you did not write there.

**Running things.**
- The shell is fish: put multi-step sequences in a bash script file in that subdirectory.
- Use `/usr/bin/python3` for the Python tools.
- Builds and tests run at their tools' default parallelism: no `-j`.

## Layout

| Path | Contents |
|---|---|
| the IDA databases | **Not in the repository**, absent in your worktree: the user's local IDA databases of the original program, `Rebirth.exe.i64` (the UI, song loading, the export driver, the RBM container) and `ReBirth.dll.i64` (the engine). IDA opens a `.i64` by itself and needs no exe or dll next to it. They are references: agents never save or modify them. They live outside the repository; the supervisor gives their path in each agent's prompt, and an agent without a path asks the supervisor and does not search the disk. Take a copy, reading only; the IDA working files (`*.id0` …) belong to the user's open IDA. The original program itself (`Rebirth.exe`, `ReBirth.dll`) is not in the repository and no test or tool may need it: the tests take the same files from the CD image (`RB_CD_ISO`, rb-testkit) and skip with a message when it is not set. |
| `docs/` | Knowledge documents, edited like source. `RBS42.txt` is the official RBS file format; `initial_investigation.md` holds the engine's facts. New facts about the original go into a document here, in the same commit as the task. `original_bugs.md` lists the defects of the original: a newly found defect goes into `docs/original_bugs.md` as well as its home document. |
| `tools/` | Python tools: the test-machine runners, the `.rbs` writer, the comparator and the like. |
| `tests/` | Test songs and reference renders (their storage is decided in a `Decide:` task). |
| Rust workspace | At the repository root once m-1 creates it: `cargo build --release`, `cargo test`. |

## IDA

Use the idalib MCP tools only. Never drive IDA from the shell, and never use the user's GUI session.

**Opening a database.**
1. Copy the database into your scratchpad subdirectory (`cp <path from the supervisor's prompt>/ReBirth.dll.i64 <scratch>/`; the databases are not in the repository, so your worktree has none, and the user's file is only read).
2. Open the copy with `idb_open(input_path=<copy>, mode="force_headless")`.
3. Close it with `idb_close(save=false)` when you are done.

**Read-only.** Do not rename, retype, comment or patch in a database that is committed: the facts you find go into `docs/` and the task's notes, with addresses (`0x10xxxxxx` is the DLL, `0x4xxxxx` the exe).

**What to trust.**
- Hex-Rays is a second opinion: check exact x87 operand order, rounding points (an `fstp dword` stores and rounds to f32) and integer semantics against the disassembly.
- Decode constants from the bytes, not from IDA's float rendering.
- Note that .data above 0x10034400 in the DLL is zero in the file: those tables are computed at run time.

## The Windows test machine

The original ReBirth runs on a Windows machine, reached with dcre's `~/dev/dcre/common/tools/rcs_client.py`. Its commands are `info`, `ls`, `get`, `put`, `rm`, `exec`, `ps`, `kill`, `input`, `screenshot`; see `--help`. Its config is `~/dev/dcre/common/tools/rcs_client.toml`. Remote paths use / separators: an absolute path (`C:/ReBirth RB-338/ReBirth.dll`) names any file on the machine, a relative one starts at the server's root (`C:\re`).

**The installation.** ReBirth 2.01 is installed in `C:\ReBirth RB-338`, outside the rcs server's root (`C:\re`). rcs reaches it with absolute paths (`rcs ls 'C:/ReBirth RB-338'`, `get`, `hash`, `exec --cwd`; dcre bd32edb). Never write into the installation: copies to change go into `C:\re\rb`.
- Start the app with `exec cmd.exe /c start "" /D "C:\ReBirth RB-338" "C:\ReBirth RB-338\Rebirth.exe"`, then find its PID with `tasklist`, and stop it with `taskkill /PID <pid>`.
- At start the app shows a modal "Failed opening MIDI input" box, window title "ReBirth RB-338". Dismiss it with `rcs_client.py input --window "ReBirth RB-338" key:enter`.

**rb's working folder.** Everything rb puts on the machine (songs, renders, tools) lives under `rb/` on the server's root, i.e. `C:\re\rb`.

**The full description.** The installation's layout and how to drive the app are written in `docs/test_machine.md`, created by task-002. Read it before you use the machine.

**Permits.** The machine is shared with other projects through `locky`, a local semaphore server with leases (`locky --help`, `locky list`). The semaphore is `redc-win-run`.
- Read the permit count from `locky list`; never assume it.
- A ReBirth session is a desktop session: it takes the keyboard and the screen. Hold **all** the permits while one runs:
  - Acquire: `locky acquire redc-win-run N --owner YOURBRANCH --acquire-timeout 3600 --lease-timeout 1800`
  - Keep the lease alive: `locky refresh redc-win-run --owner YOURBRANCH --lease-timeout 1800`
  - Release when done: `locky release redc-win-run all --owner YOURBRANCH`
- An rb runner tool, once one exists, takes and releases the permits itself.
- Never `locky release-all`.
- Never stop a process you did not start: stop only your own, by the id rcs gave you.

## Testing

- The Rust code: `cargo test`, with every test passing.
- The Python tools: `/usr/bin/python3 -W error::ResourceWarning -m unittest discover -s tools -p '*_test.py'` and the same with `-s tools/rbhost` (the test files are named `*_test.py`, so plain `discover` finds none).
- A change to the engine's output is checked against the reference corpus with the comparator tool (m-0). The commit message states the result: exact, within the tolerance, or the first differing sample.
- A tool change is tested by running it on at least one song, with what ran in the notes.
- A docs-only change needs nothing.

## Rebase and the final check

Rebase once onto main after your tests pass and before the final commit.

**Conflicts.**
- A file your task did not change takes main's version.
- A file you changed is merged hunk by hunk, with both sides kept.
- A backlog file is never resolved by hand: take main's, then redo your task's changes with the CLI.

**The mandatory check.** `git diff main --stat` must list only files your task changed. Reset anything else to main (`git checkout main -- <path>`) and amend. Say in your report that you ran this check.

## Rules that apply to everyone

- **Edit with Edit/Write.** Source is edited with the Edit/Write tools, not sed or python.
- **Don't expand the task.** A design decision the task does not take, or a change outside its area, becomes a `Decide:` task for @pingw33n with a dependency, and you stop there.
- **Verify before acting.** Check a claim from written evidence (including `docs/initial_investigation.md`, which was written from a survey and marks guesses with [G]) against the binary before you rely on it. List what you could not confirm.
- **Report through the backlog.** Your final chat message is the task id plus what the supervisor must do (merge, decide). Nothing is reported only in chat.
- **Spelling.** Spell "color" the American way.
