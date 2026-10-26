---
id: TASK-029
title: Golden test suite over the reference corpus
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
labels:
  - tests
  - engine
milestone: m-1
dependencies:
  - TASK-028
  - TASK-014
  - TASK-025
  - TASK-026
  - TASK-027
priority: high
type: task
ordinal: 29000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The golden test suite: a cargo test (behind a feature or an ignored-by-default flag if the corpus is not in the repository) that renders every corpus song with the decided initial-state preset and compares it with its reference by the decided pass criterion, printing the first difference (frame, time, bar) on failure, plus a summary command (`rb compare` or tools/rbcmp.py over the whole corpus) for agents. The milestone is done when every song passes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 cargo test runs the whole corpus comparison and reports the first difference per failing song
- [ ] #2 Every corpus song passes the pass criterion
<!-- AC:END -->
