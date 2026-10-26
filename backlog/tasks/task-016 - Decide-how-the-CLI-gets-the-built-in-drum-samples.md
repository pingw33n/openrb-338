---
id: TASK-016
title: 'Decide: how the CLI gets the built-in drum samples'
status: To Do
assignee:
  - '@pingw33n'
created_date: '2026-10-26 09:06'
labels:
  - decision
  - engine
milestone: m-1
dependencies: []
priority: medium
type: spike
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The engine needs the 46 built-in drum samples (AIFF resources of type "AIFF", IDs 1000..1045, in the .rsrc of ReBirth.dll; docs/initial_investigation.md section 7.4). They are Propellerhead's data. How does the `rb` CLI get them?

Options:
1. At run time from the user's ReBirth.dll (`--rebirth-dll PATH`, or a default location): nothing of Propellerhead's is redistributed; the CLI needs a ReBirth installation.
2. Extracted at build time from original/ReBirth.dll and embedded (`include_bytes!`): self-contained binary, redistributes the samples.
3. Extracted once by a tool into a data folder next to the binary.
Recommendation: 1, with a resource parser in Rust (no Windows API), and the tests reading original/ReBirth.dll.
<!-- SECTION:DESCRIPTION:END -->
