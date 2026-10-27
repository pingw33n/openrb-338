---
id: TASK-088
title: 'Player and UI: a song whose load the node pool refuses is not opened'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-27 20:22'
updated_date: '2026-10-27 21:22'
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
- [x] #1 Player::build or Player::load reports a refused load instead of queueing the song
- [x] #2 rb-ui shows 'There is not enough memory for opening the Song.' and keeps the current song; rb play exits with the error
- [x] #3 A unit or integration test loads a generated song of more than 100000 events and checks it
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. rb-engine: LoadRefused::ALERT const (the exe's text). 2. rb-player: build -> Result<Box<Engine>, LoadRefused>; load -> Result<(), LoadError{Refused,Queue}>; test with a generated 101000-event song. 3. rb-ui: Link::load returns Result before touching panel/kit/backlog; App::adopt shows ALERT in notice, keeps doc/panel, no startup sound; start() likewise. 4. rb play: exit 1 with the refusal. 5. tests, clippy, fmt.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a90771e36cd50ddb6

Done: LoadRefused::ALERT (rb-engine); Player::build -> Result<Box<Engine>,LoadRefused>; Player::load -> Result<(),LoadError{Refused,Queue}>; Link::load -> Result<(),LoadRefused> (builds first, so panel/kit/backlog untouched on refusal); App::adopt shows ALERT in notice, returns before mod_opened and doc change (covers start too); rb play fails exit 1 with the refusal text. Tests: crates/rb-player/tests/load_refused.rs, crates/rb-ui/tests/menu.rs a_song_the_node_pool_refuses_is_not_opened (open path and start path, generated 101000-event song in a tempdir). rb play exit path not run on a real big file (needs audio device; the code is the load() error map before output start). Full cargo test --release all pass, clippy -D warnings clean, fmt clean.

Review round 1: send back for the commit message only (subject 78 chars, Testing line without a count). Amended: subject shortened, cargo test 450 pass.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Player::build/load report the node pool's refusal; rb-ui shows 'There is not enough memory for opening the Song.' and keeps song, panel, document, no startup sound; rb play exits 1 with the error. Tests added in rb-player and rb-ui; cargo test --release all pass, clippy and fmt clean. Branch worktree-agent-a90771e36cd50ddb6.
<!-- SECTION:FINAL_SUMMARY:END -->
