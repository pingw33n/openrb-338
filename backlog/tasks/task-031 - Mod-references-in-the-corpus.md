---
id: TASK-031
title: Mod references in the corpus
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
labels:
  - rbm
  - tests
  - testmachine
milestone: m-2
dependencies:
  - TASK-014
  - TASK-030
priority: medium
type: task
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Collect real RBM mods (from the test machine's installation or from the user; ask through a Decide task if none are available) and add mod songs to the corpus: songs naming each mod in GLOB, rendered by the decided capture protocol (rbhost with the msg 55 bank load, or the app's export), stored and listed in the corpus MANIFEST.md like the rest.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 At least three mods with songs using them are in the corpus with their references and manifest entries
- [ ] #2 Re-capturing one mod reference equals the committed one
<!-- AC:END -->
