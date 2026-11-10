---
id: TASK-177
title: 'Decide: how orb-mcp and the beats cookbook go back to the community'
status: Done
assignee:
  - '@pingw33n'
created_date: '2026-11-10T13:44:00Z'
updated_date: '2026-11-10T13:53:00Z'
labels:
  - decision
  - docs
dependencies: []
priority: high
type: spike
ordinal: 193000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The fork author (gaolivaresm) built orb-mcp (an MCP server over orb-cli, six tools including orb_build and orb_song), docs/beats.md (a measured cookbook for writing songs) and examples/ (one piece). It all works and is pushed to the fork branch feat/orb-build. What is not decided is how it reaches upstream.

The fork also carries seventeen commits on feat/orb-mcp that were made before the backlog flow (docs commits in sequence, no task each). They contain the same content as feat/orb-build plus older intermediate states; branching feat/orb-build from main was the clean way to present the same work as one task.

Options, with a recommendation:
1. A pull request from feat/orb-build against upstream main, one commit, task-176 as the description. Recommended: the work is a single coherent task and the branch is a single commit, which is what AGENTS.md asks for. Risk: upstream may not want a Python subprocess dependency in a Rust server (the merge could be ported to Rust later, in a task of its own).
2. Ask upstream first with an issue describing what orb-mcp is and whether they want it, before opening the PR.
3. Keep it in the fork and publish the docs/teaches as an article instead.

Questions this task must settle:
- is the Python subprocess acceptable for upstream, or must the merge be ported to Rust first?
- does upstream want examples/ in the repository at all?
- who reviews, given the reviewer agent is defined for backlog tasks in this repository?

At stake: whether the fork stays a personal experiment or becomes part of the project.
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
The Python question, spelled out (this is the part worth deciding first).

orb_build as built wraps tools/rbs.py over a subprocess, because the spec merge and the validation live only in Python. That gives one definition of the format instead of two, and makes the built file byte-identical to rbs.py build by construction. The cost: a Rust binary that needs Python 3 and a tools/rbs.py next to it, which is fine for a test (the project already requires Python for tests) and wrong for a distributed binary.

Three ways out, and my recommendation changed after writing them down:

A. Keep the subprocess. Cheapest, and the byte-identity is guaranteed by construction. Accepts a Python dependency in a shipped server, and ORB_RBS_PY as the escape hatch.

B. Port the merge and the validation to Rust. Removes the dependency and keeps the nice spec form, but creates a second definition of the format. Two definitions eventually disagree, which is the one failure mode this fork spent its whole length avoiding.

C. Change the tool so it needs neither. orb_build receives the COMPLETE model (not a spec to merge) and writes it with rbs::Song::write(), which already exists in Rust and is already tested by crates/rbs/tests/corpus.rs against the corpus. The agent reads a song with orb_song, changes the fields it wants, and writes it back. No merge, no validation beyond parse/write, no Python, no duplicate definition.

Recommended: C. It is the smallest surface, it uses code that exists and is tested, and it removes the dependency and the duplication at once. What it loses is the convenience of a partial spec ("just change the tempo"): the client has to send the whole model back, which orb_song already gives it, and which is what an agent does naturally anyway.

If C is chosen, the spec-merge form can still be offered to humans through tools/rbs.py, which is where it already lives.

Outcome (2026-11-10): option A for now, option C proposed in the PR as future work.

Why A after all: the premise that made C urgent was wrong. orb-mcp is not in any packaging script or CI workflow (no mention in scripts/package*.sh, none in .github/workflows), so no Python-needing binary is distributed today. Whoever wants the server builds it from the checkout, where tools/rbs.py is always next to it. The subprocess is therefore not a defect of the shipped product, because there is no shipped product yet.

Why C is still worth proposing, and not doing unilaterally: writing it for real means adding flux Serialize to the fourteen structs of crates/rbs (Song, Head, Glob, Usri, Mixer, MixChannel, Params, Step303, Pattern303, Tb303, Drums, Event, Track, Unknown), because rbs has serde_json only as a dev-dependency and nothing in the crate is serialisable. That is a change to a crate the upstream maintainer owns, in the middle of a PR about a different crate. It is the kind of change that should be the maintainer's call, or its own task.

So the PR describes C as an option with its cost written down, and leaves the decision upstream:

- A today: orb_build runs tools/rbs.py. Byte-identical to rbs.py build by construction, no second definition of the format, requires Python and the tool next to the binary.
- C later: derives Serialize on the rbs model, orb_build takes the whole model and writes it with Song::write(), which already exists and is already tested by crates/rbs/tests/corpus.rs. No merge, no Python, no duplicate definition; the cost is a change in rbs and the loss of the partial-spec convenience (the client sends the whole model back, which orb_song already gives it).

No further work on this task until the maintainer answers the PR.
<!-- SECTION:NOTES:END -->
