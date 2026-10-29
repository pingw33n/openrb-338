---
id: TASK-147
title: Fix the wrong loop ranges (song ends) in the song collection
status: To Do
assignee:
  - '@pingw33n'
created_date: '2026-10-29 14:56'
labels:
  - tests
dependencies:
  - TASK-129
references:
  - tests/refs/thirdparty_songs.json
  - crates/orb-cli/tests/thirdparty.rs
  - tools/rbhost/thirdparty_songs.py
  - docs/rbm.md
priority: low
type: task
ordinal: 165000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked on 2026-10-29 for a task of their own to fix the songs in their collection (~/rb-songs/songs, read only for agents) whose loop range is set wrong. orb render, like ReBirth's "Export Loop as Audio File", renders the loop range whether the loop switch is on or off, so a wrong loop cuts the export short.

The supervisor's survey of 2026-10-29: tools/rbs.py and tools/rb1x.py were run over every readable song, and the track events were counted outside the loop range [loop start, loop end), not counting a single event exactly at the loop end. Of 1238 readable songs:
- 228 likely truncated: events in at least 8 bars after the loop, and in at least as many bars as the loop is long.
  - By loop length: 1 bar 90, 2-4 bars 41, 5-16 bars 20, 17-64 bars 9, over 64 bars 68.
  - 67 loops start after bar 0, and 148 of the 228 have the loop switch off.
  - The most common loops are the defaults 0+1 (53), 0+4 (32) and 0+8 (8), set by no one.
- 329 borderline: events in 4 or more bars after the loop, but fewer than the loop's length.
- 681 fine: 0 to 3 bars of events after the loop.
Caveats: every track event counts, so knob automation alone can flag a song. Several flagged songs end at exactly bar 500 or 999, which may be filler, not music. 35 songs carry events past bar 999, which the original never plays.

The list, sorted worst first, with loop start, length, loop switch, format, bars with events after, events after, last event bar and path, is in the supervisor's scratchpad: /private/tmp/claude-501/-Users-pingw33n-dev-openrb-338/8f24950b-b10f-47b2-be09-4ad0d95914c5/scratchpad/trunc/truncated.tsv (the scratchpad is temporary: copy it somewhere lasting if needed, or ask the supervisor to regenerate it).

What (the user's work, in ReBirth or OpenRB-338):
- For each song, set the loop to the song's real start and end (or decide the song is right as it is), and resave it.
- Resaving changes a song's SHA-256, so the task-129 manifest (tests/refs/thirdparty_songs.json) must be regenerated with tools/rbhost/thirdparty_songs.py on the test machine afterwards. Ask the supervisor; the gated songs test fails on any changed file until then.
- Optional: ask the supervisor to re-run the survey after the fixes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every song of the truncated list is fixed or judged right as it is
- [ ] #2 The task-129 manifest is regenerated for the resaved songs, and the gated songs test passes
<!-- AC:END -->
