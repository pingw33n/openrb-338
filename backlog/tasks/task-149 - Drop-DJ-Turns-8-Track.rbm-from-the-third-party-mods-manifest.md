---
id: TASK-149
title: Drop DJ Turn's 8-Track.rbm from the third-party mods manifest
status: To Do
assignee:
  - '@agent-chore'
created_date: '2026-10-29 15:06'
updated_date: '2026-10-29 15:06'
labels:
  - tests
milestone: m-4
dependencies:
  - TASK-129
references:
  - tests/refs/thirdparty_mods.json
  - tools/thirdparty_mods_manifest.py
  - crates/orb-cli/tests/thirdparty.rs
  - docs/rbm.md
priority: medium
type: chore
ordinal: 167000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: on 2026-10-29 the user removed "DJ Turn's 8-Track.rbm" from their folder of recrypted third-party mods (ORB_THIRDPARTY_MODS_DIR=~/rb-mods/mods, now 171 files). It was a byte-identical copy of 8tRack.rbm, with the same INFO name "8tRack". The gated test `cargo test --release -p orb-cli --test thirdparty` (the mods half) now fails with "DJ Turn's 8-Track.rbm: not in ~/rb-mods/mods", because tests/refs/thirdparty_mods.json still lists it. The user chose to drop that one entry rather than regenerate the manifest, which needs the original mods and the DAT.

What:
- Remove exactly that one entry from tests/refs/thirdparty_mods.json, and change nothing else in the file. If tools/thirdparty_mods_manifest.py has an option to drop or prune entries without the originals, use it. Otherwise remove the entry with the Edit tool, keeping the file's formatting, and say so in the notes.
- Update every count that says 172 mods where it means this folder: docs/rbm.md section 11, AGENTS.md, the test's messages or comments, and the task-129 docs (git grep "172").
- If the duplicate-name note in docs/rbm.md 11.4 (5 pairs, including 8tRack) describes the folder as it is now, say that 8tRack's duplicate was removed by the user on 2026-10-29 (4 pairs remain). Keep the facts about the original's behavior.
- Run the gated mods test with ORB_THIRDPARTY_MODS_DIR=~/rb-mods/mods (read only) and ORB_CD_ISO="/Users/pingw33n/Downloads/Rebirth 2.01 (2001 version).iso": it must pass with 171 mods.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tests/refs/thirdparty_mods.json lists 171 mods, without DJ Turn's 8-Track.rbm, and nothing else in it changed (git diff shows only that entry)
- [ ] #2 The gated mods test passes on ~/rb-mods/mods; the counts in docs and messages say 171 where they mean the folder
<!-- AC:END -->
