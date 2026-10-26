---
id: TASK-019
title: 'Engine tables, generated and checked against the original''s dumps'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
labels:
  - engine
milestone: m-1
dependencies:
  - TASK-015
  - TASK-011
priority: high
type: feature
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The engine's lookup tables: the ones computed at init (T and PAN, 303 filter table FILT and pitch table, PCF LP/BP tables, distortion waveshaper W, T808, T909, DT, the cutoff-smoother and PCF constants) and the static ones read from the DLL's data (distortion grid, PCF patterns/lengths/resolutions/decays, 808/909 tone, decay and attack tables, SEQ808/SEQ909, 303 tune table). Formulas and addresses: docs/initial_investigation.md sections 4, 6, 7, 8. Generate the computed tables in Rust (f64, f32 stores, the CRT pow and x87 transcendentals approximated by Rust's) and compare each against rbhost's --dump-tables output in a test; where an entry differs, find whether a faithful x87/CRT emulation fixes it, else embed the dumped table (as a generated Rust source with a comment saying why). Static tables: read their bytes from original/ReBirth.dll in a test and assert the Rust constants equal them.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every table is bit-identical to the original's dump (f32 bits), checked by cargo test
- [ ] #2 Each embedded (not generated) table has its reason in a comment and in the task's notes
<!-- AC:END -->
