---
id: TASK-088
title: 'Player and UI: a song whose load the node pool refuses is not opened'
status: To Do
assignee: []
created_date: '2026-10-27 20:22'
labels:
  - ui
  - cli
  - engine
milestone: m-3
dependencies:
  - TASK-083
references:
  - crates/rb-player/src/lib.rs
  - crates/rb-ui/src/link.rs
  - crates/rb-cli/src/main.rs
  - docs/song_load.md
priority: low
type: enhancement
ordinal: 108000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found by task-083 (2026-10-27). When ReBirth.dll refuses a song-load msg-37 batch (sub_10021010: more events than free nodes in the 100000-node pool, reply ['K', 22]), Rebirth.exe throws XOutOfMemory (0x436E95), turns it into XSongOutOfMemory (sub_425C4D, catch 0x426430), shows the STOP alert 'There is not enough memory for opening the Song.' (sub_40B575 catch 0x40B778, alert id 0x1DCF592A) and opens no song window (docs/song_load.md section 5.2). rb-engine models the refusal: Engine::load returns Err(LoadRefused) and keeps it (Engine::load_refused), and rb render fails with that text. Player::build (crates/rb-player/src/lib.rs) still returns the engine with the batches before the refused one, and rb-ui's Link::load (crates/rb-ui/src/link.rs) and 'rb play' (crates/rb-cli/src/main.rs) queue it as the newest song. Do: make Player::build / Player::load report the refusal (for example a Result with the LoadRefused), make rb-ui show the exe's alert text and keep the current song and panel, and make rb play exit with the error. Only songs of more than about 100000 events reach it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Player::build or Player::load reports a refused load instead of queueing the song
- [ ] #2 rb-ui shows 'There is not enough memory for opening the Song.' and keeps the current song; rb play exits with the error
- [ ] #3 A unit or integration test loads a generated song of more than 100000 events and checks it
<!-- AC:END -->
