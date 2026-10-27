---
id: TASK-057
title: 'RE + engine: recording (Record button, msg 35 id 2, the record hook)'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-27 07:34'
labels:
  - re
  - engine
milestone: m-3
dependencies:
  - TASK-043
references:
  - docs/initial_investigation.md
  - crates/rb-engine/src/seq.rs
  - crates/rb-player/src/lib.rs
priority: low
type: feature
ordinal: 52000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the transport's Record button (docs/ui_spec.md section 3) toggles globals byte 2 (msg 35 id 2, sub_10021194) and in song mode records the panel's controller changes into the tracks. rb stores the byte (Sequencer::set_record) and models its effect on locate, but not the recording: the record hook sub_10021340 (called by the device messages, e.g. the 303's msg 40 at 0x10025CFC via sub_10012BC0; acts when globals byte 0 and byte 2 are set: it sets the track's hold byte +4+ctrl and inserts the event, quantised with sub_100146E0 while playing or sub_10014720 otherwise, loop-end adjusted, with sub_10014750 into a list), the merge when record is turned off or the loop wraps (sub_1001F520, Sequencer::tick's comment at 0x1001F0C7), and the hold mask's effect on the clock. The real-time player (task-043) therefore has no Record command.

Do: read the record path in ReBirth.dll (sub_10021194, sub_10021340, sub_1001F520 and the list it records into), document it with addresses in docs/initial_investigation.md section 5 (update its [G] items) and docs/export_state.md where it touches the transport, implement it in rb-engine's sequencer (allocation: the recorded events grow a Vec; the callback must not allocate, so reserve capacity up front or record into a fixed buffer, and say which), add Command::Record to rb-player, with tests including crates/rb-player/tests/no_alloc.rs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The record path is documented with addresses and its [G] items in docs/initial_investigation.md section 5 are confirmed or refuted
- [ ] #2 rb-engine records as the original does and rb-player has a Record command, with tests; the callback stays allocation-free
<!-- AC:END -->
