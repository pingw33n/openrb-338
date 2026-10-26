---
id: TASK-033
title: Mods in the engine and the rb CLI
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
labels:
  - rbm
  - cli
milestone: m-2
dependencies:
  - TASK-032
  - TASK-031
  - TASK-029
priority: medium
type: feature
ordinal: 33000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Use mods in the engine and the CLI: the song's GLOB mod name ("Standard ReBirth" means the built-in kit) selects a mod found in `--mod-dir DIR` (by the INFO name or file name, as the original resolves it), `--mod FILE` overrides it, and a missing mod is an error unless `--standard-kit` is given (the original offers Download/Standard/Cancel). The golden suite covers the mod songs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 rb render uses the song's mod by name from --mod-dir, --mod overrides it, and a missing mod behaves as described
- [ ] #2 Every mod song of the corpus passes the golden suite
<!-- AC:END -->
