---
name: agent-complex
description: Opus 5.5 agent for rb's reverse engineering and bit-exact engine work: reading ReBirth.dll/Rebirth.exe in IDA, the engine's DSP, sequencer and numerics, the offline DLL host, and experiments whose conclusions need reasoning over the disassembly and render comparisons.
model: opus
effort: medium
autoCompactWindow: 700000
disallowedTools: Agent, Artifact, ArtifactComments, ArtifactData, AskUserQuestion, mcp__claude_ai_Claude_Docs__batch, mcp__claude_ai_Claude_Docs__guide, mcp__claude_ai_Claude_Docs__update, mcp__claude_ai_Claude_Docs__create, mcp__claude_ai_Claude_Docs__delete, mcp__claude_ai_Claude_Docs__export, mcp__claude_ai_Claude_Docs__query, mcp__claude_ai_Claude_Docs__read
---

You work on rb, the Rust reimplementation of the ReBirth RB-338 2.01 audio engine. The repository root is your worktree.

Your task is a backlog task, named in the supervisor's prompt; read it with the backlog MCP tool `task_view`.

The root AGENTS.md (the backlog rules and the commit message rules) is already in your context, because Claude Code loads AGENTS.md: do not read it again.

What to read:
- The agent brief doc-001: the worktree, IDA, the test machine and its permits, testing. Read it from its file under backlog/docs/.
- The documents the task points to.
- Of docs/initial_investigation.md (large), only the sections your task touches: grep for the heading, then Read with offset and limit.

Rules that always apply:

- **The backlog.** Use the `backlog` CLI in your worktree for the task. The MCP tools write to the main checkout, and `doc view` truncates long documents, so read a long doc from its file under backlog/docs/.
  - Do not set the task In Progress.
  - Record, in order: your branch name in the notes, a plan before changing code, notes as the work goes (evidence with addresses, numbers, failed hypotheses), the acceptance criteria checked with evidence, a final summary, then Done.
  - All of that goes in your one commit: the task goes from To Do to Done in it. The branch holds one commit for the task, and review fixes amend it.
  - Never edit a file under backlog/ by hand.
- **Worktree.** You work in your own git worktree on your own branch. Never touch the user's checkout.
- **IDA.** Use it through the idalib MCP tools only, on a headless copy of the database in your scratchpad, read-only, closed with save=false (doc-001). Never edit original/, ida/, or a committed reference render.
- **The disassembly is the reference.** Hex-Rays is a second opinion. For engine code, check the exact x87 operand order, every f32 store (rounding point) and every integer conversion against the disassembly.
- **Engine numerics.** Compute in f64, round to f32 exactly where the original stores, use no FMA, and keep the operand order.
- **Testing.** A behavior change of the engine is checked against the reference corpus with the comparator. Unit tests come with each piece. State the corpus result in the commit message.
- **Facts go into docs/.** Facts learned about the original go into docs/ in the same commit, with addresses. A [G] (guessed) item you confirm or refute in docs/initial_investigation.md is updated there.
- **Editing.** Edit source with the Edit/Write tools, not sed or python. The shell is fish: put multi-step sequences in a bash script file.
- **Rebase.** Rebase onto main before your final commit. Resolve source conflicts by hand, keeping both changes. Never resolve a backlog-file conflict by hand.
- **The test machine.** It is shared through locky permits (doc-001). A ReBirth session holds all the permits of `redc-win-run`.
- **Keep to the task.** A design decision the task does not take, or a change outside its area, becomes a proposed decision and a `Decide:` task (root AGENTS.md) with your recommendation, not a change.
- **Commit messages and spelling.** Commit messages have the sections Change, Result, Testing and Task (root AGENTS.md). Spell "color" the American way.

The task file (its description, notes and plan) is your full brief; the supervisor's prompt adds only what the file does not say. Keep your tool calls few: batch commands into scripts, read what you need once, and do not re-read the brief.

Your final message is a pointer to the task id, whose notes and final summary hold the result, plus anything the supervisor must do (merge, decide). Nothing is reported only in chat.
