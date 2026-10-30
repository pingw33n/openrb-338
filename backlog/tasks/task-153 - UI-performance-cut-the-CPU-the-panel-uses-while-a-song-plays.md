---
id: TASK-153
title: 'UI performance: cut the CPU the panel uses while a song plays'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-30 08:15'
updated_date: '2026-10-30 15:10'
labels:
  - ui
milestone: m-5
dependencies: []
references:
  - crates/orb-ui/src/app.rs
  - crates/orb-ui/src/art.rs
  - crates/orb-ui/src/link.rs
  - docs/ui_spec.md
priority: medium
type: enhancement
ordinal: 171000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user saw OpenRB-338 use about 40% CPU on macOS (2026-10-30). The supervisor measured it the same day, in a release build on an Apple-silicon Mac:
- with "Alloy Cowboy Resurrection.rbs" open in a maximized window, the app used 1.4 s of CPU in 10 s (about 14%);
- with no song, the app used 0.04 s in 10 s;
- `sample` showed the CoreAudio output thread (cpal-coreaudio-default-output) essentially idle;
- the time is on the main thread in eframe's run_ui_and_paint: orb_ui::app::App::ui building the panel, egui tessellation, and egui_wgpu paint and wgpu command encoding, plus the Metal queues and the window server.

The engine is cheap: `orb render` does a five-minute song in about 1.4 s, about 0.5% of one core in real time. So while playing, nearly all the CPU is the UI redrawing the whole panel at 60 fps: crates/orb-ui/src/app.rs:510 requests a repaint every 16 ms whenever a player link exists. egui is immediate mode and egui_wgpu repaints the whole surface each frame; it has no dirty rectangles. The user cares about the playing case. The idle case (no repaint needed while stopped) is welcome, but secondary.

What, in order, measuring after each step:
1. Measure first. Record CPU% (ps or `top -l`, and the cpu time over a fixed 20 s) while playing a busy song at the startup window size and maximized, with the frame time from egui's or eframe's timing or a puffin profile. Record per-frame counts: shapes, meshes, vertices and texture binds. Keep the numbers in the notes.
2. Repaint only as often as something changes. The engine's snapshot (step LEDs at sixteenth notes, the song position, meters) changes far less often than 60 times a second. Request a repaint when link.sync reports a visible change, at most 30 fps for the meters (the original's own panel refresh rate is worth checking in docs/ui_spec.md or the disassembly before choosing), and none while stopped with nothing animating (LED fades, held buttons, the logo animation and an export keep their own requests).
3. Make a frame cheaper, if steps 1-2 leave it significant: cache the static layer (background, frames, labels: everything that does not change while playing) into an offscreen texture once per size and scale change, through an egui_wgpu paint callback or a TextureHandle rendered from the SVG parts. Each frame then draws one textured rectangle plus the moving parts (knobs, sliders, LEDs, displays, meters). Also look for per-frame waste in App::ui: allocations, text layout redone every frame, and SVG parts re-tessellated instead of drawn as cached textures.
4. Keep the look: a before/after screenshot pair (the hidden --screenshot option, docs/building.md) at two scales must be pixel-identical, or each difference must be explained.

Out of scope: true partial presentation (damage regions) or a renderer outside egui.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Baseline and final CPU while playing (startup size and maximized) and the frame time are recorded in the notes, with the commands
- [x] #2 Playing a song uses substantially less CPU than the baseline (target: half or less); stopped with nothing animating, the app does not repaint continuously
- [x] #3 The panel looks the same: the before/after screenshots at two scales are pixel-identical, or the differences are explained; while playing the window still draws about 28 frames a second and a headless test shows the position, the running lights and the meters following the snapshot frame by frame (the by-eye check of the playing panel is deferred to a supervisor follow-up)
- [x] #4 fmt, clippy and the orb-ui tests pass; docs/ui_spec.md states the repaint rule
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Measure: hidden --play option (press Play at the start), ORB_UI_FRAME_STATS=1 (frames/s and eframe cpu_usage per frame), a headless frame-cost test (shapes, meshes, vertices, textures, show and tessellate time), CPU time over 20 s with ps -o time= (native 624x510 and maximized, playing and stopped), paired runs of old and new binaries against machine-load noise.
2. Repaint rule: Link::moving (playing, a lit meter, changed gain-reduction LEDs, commands not yet in the snapshot) repaints at 30 fps (METER_FRAME 33 ms); otherwise no request; the 303 note flash schedules its own end. Measure.
3. Cheaper frame if still above half: reorder the panel's shapes so that same-texture shapes are adjacent (epaint merges consecutive meshes of one texture and clip into one draw call), moving a shape only past shapes it does not overlap (pixel-identical by construction). Measure; look at per-frame waste in App::show.
4. Screenshots before/after at two scales with --screenshot, compared pixel by pixel; a frame sequence of the playing panel by eye.
5. docs/ui_spec.md section 16: the repaint rule; docs/building.md: --play and ORB_UI_FRAME_STATS. Tests, fmt, clippy.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a00a2a667598a3060

Measurement setup (2026-10-30, Apple-silicon Mac, release build). Song: tests/songs/shipped/Trakka Rekka.rbs (built-in kit, song mode, 139 BPM; Alloy Cowboy Resurrection needs the Pitch Black mod, which opens an error box over the panel, so it was not used).
Scripts in the scratchpad task153/: measure.sh LABEL BIN ARGS (launches ORB_UI_CONFIG=<scratch>/cfg/ui.toml BIN ARGS SONG, waits 5 s, ps -o time= before and after a fixed 20 s, kills it), all.sh (playing/stopped x --size 624x510/maximized), paired.sh A B N (alternates two binaries N times, against load noise).
Added for measuring: hidden --play (presses Play once the song is open), ORB_UI_FRAME_STATS=1 (every 2 s: frames/s and the mean eframe cpu_usage per frame = App::ui + tessellation + wgpu paint without vsync wait), tests/frame_cost.rs (headless: shapes, meshes, vertices, textures, show and tessellate time; cargo test --release -p orb-ui --test frame_cost -- --nocapture).

Step 1, baseline (main 74cbe68 + --play):
- single runs (load average 3-5): playing native 25.6%, playing max 26.2%, stopped native 25.2%; repeated 18.0-19.7%; the machine load makes single runs noisy, so the comparisons below are paired.
- paired, playing, native 624x510: 18.4 / 18.6 / 18.7% (3.68-3.74 s per 20 s); maximized: 19.7 / 19.5%.
- frame stats: 57-58 frames/s, 1.9-2.1 ms CPU per frame native, 2.2-2.4 ms maximized; stopped with a song also 57.7 frames/s (the 16 ms request whenever a link exists).
- headless frame cost (demo panel): 559 shapes, 524 meshes (draw calls), 14112 vertices, 112 textures at native 1x; 534 meshes, 16412 vertices, 161 textures at 1800x1100 2x; App::show 0.23 ms, tessellate 0.11 ms.
- sample(1) of the playing app: the main thread is idle 92% of samples; of the busy part, egui_wgpu Painter::paint_and_update_textures about half (update_buffers, queue submit, get_current_texture, per-draw bind group state), App::ui a third (Image::paint_at -> loader lookups; resvg rasterizing new part states at first sight), tessellation a tenth.
- The original's panel refresh: the Rebirth.exe database's import list (copy, read only, closed unsaved) shows only KERNEL32 (Sleep and others), no SetTimer, so its refresh rate was not found this way; 30 fps was chosen as the task suggests.

Step 2, repaint rule (Link::moving + METER_FRAME 33 ms + Panel::next_change):
- first try repainted at 29 fps while stopped: s.pending (a loaded song waits for Play) is true for the startup song until Play, and the compressor's gain-reduction LEDs hold [0,0,0,2,1,...] in silence (debug print). Fixed: pending dropped (a waiting song is not followed, so it moves nothing), the GR LEDs count only when they change.
- paired, playing, native: 11.3 / 11.5 / 11.5% (2.27-2.31 s); maximized 12.3 / 12.1%. About 62% of the baseline: not yet half.
- frame stats playing: 28.7-29.0 frames/s, 2.3 ms (native) to 2.7-3.1 ms (max) per frame.
- stopped (native and maximized): 0.16 s in 20 s (0.8%), no frame drawn (no frame-stats line).

Step 3, cheaper frames (each measured in paired runs against the binary before it; the machine was under load from other agents, load average 3-4, so only paired numbers are compared):
- Batching by texture (reorder the panel's shapes so same-texture shapes are adjacent, moving a shape only past shapes it does not overlap): draw calls 524 -> 201 headless, screenshots identical, but no CPU gain (paired, loaded: s2 14.3/14.4% vs batched s3 15.7/15.2%; App::show +0.1 ms for the ordering). Dropped. Failed hypothesis: the draw-call count is not the cost.
- Experiments (temporary env switches, removed): every part drawn with one texture 14.5% vs 15.3%, every part 10x10 points 14.0% (fill area and texture binds hardly matter); the panel not drawn at all, menu bar only, 6.0% (so the panel's content is most of the frame); the knob arcs not drawn 11.2/11.5% vs 14.1/14.7%: the arcs, 71 paths with 10452 of the 14112 vertices (headless count by shape type), cost about a fifth.
- Offscreen panel cache with dirty regions (panel rendered into a texture of the surface's format through egui_wgpu's Renderer, only the regions of changed shapes redrawn each frame, the window shows one quad): screenshots byte-identical, but no CPU gain (paired s7 plain 10.8/10.5% vs cached 11.9/11.1%): its own submit (CommandEncoder::finish, update_buffers, queue submit: ~190 of ~700 main-thread samples) costs as much as the draws it saves. Dropped (kept out of the branch).
- Kept: (a) the part textures are found in a per-context map by (part URI, pixel size) instead of egui's loaders, whose include_bytes and URI lookups ran for every part every frame (paint_part_with); (b) each knob arc is tessellated once per look (center, radius, angles, width, color, density, clip) with ctx.tessellate and drawn as that Arc<Mesh>; egui appends a Shape::Mesh unchanged, so the vertices are the ones the frame tessellator made; the points are built only on a cache miss. Headless frame cost: App::show 0.23 -> 0.17 ms, tessellate 0.11 -> 0.08 ms.
- sample(1) of the final build (10 s, playing, native): run_ui_and_paint = App::ui ~293 (paint_part_with 82, egui interact 45, menu bar 23), paint 154 (update_buffers 105, submit 45), tessellate 98 samples.

Final numbers (s8 = this branch), paired with the baseline binary, playing Trakka Rekka, 20 s each, 2026-10-30 16:27-16:32:
- native 624x510: base 20.7 / 21.2 / 20.1% (4.02-4.24 s), s8 11.2 / 10.8 / 11.9% (2.16-2.38 s): 55% of the baseline.
- maximized: base 22.1 / 22.6 / 23.0% (4.43-4.60 s), s8 11.2 / 10.9 / 11.8% (2.19-2.37 s): 50%.
- frame stats s8: 27.7-28.1 frames/s, 2.7 (native) / 3.5 (maximized, loaded machine) ms CPU per frame; baseline 57-58 frames/s.
- stopped (song loaded, not playing): base 4.6-5.1 s in 20 s (57.7 frames/s); s8 0.18-0.20 s in 20 s, no frame drawn after the startup (no frame-stats line).
- The screen of the Mac was locked from about 16:20 (screencapture shows the lock screen); the app kept drawing at 28 frames/s (frame stats) and the paired runs before and after agree (base 18.6-19.7% unlocked at 15:40, 20-23% later), so the comparisons hold, but the by-eye check of the playing panel could not be made after that. A capture of the playing panel at 15:30 (scratch p-1.png) shows the Play LED lit and the step and pattern lights following.

Screenshots (--screenshot, ORB_UI_CONFIG in the scratchpad): --demo --size 624x510 at --ppp 1 and 2 (panel scale 2 and 1) and Trakka Rekka --size 900x700 at --ppp 1 and 2 (scales 2.88 and 1.44): base and s8 PNGs byte-identical (cmp), all four; also identical for the batched and the cached builds.

Commands: scratch task153/measure.sh, all.sh, paired.sh "base s8" 3 [--size 624x510], step.sh TAG (build + four measurements with ORB_UI_FRAME_STATS=1), cmpshots.sh base s8, sample.sh + children.sh (sample(1) breakdown), cargo test --release -p orb-ui --test frame_cost -- --nocapture.

Tests: crates/orb-ui/tests/repaint.rs (Link::moving: stopped and silent false, Play sent true, playing true, stopped again false; a still panel with no audio stream asks for no repaint (> 60 s; the baseline asked for 16 ms); a lit 303 note asks for a repaint by its end), tests/frame_cost.rs (prints the counts), the --play parse in main.rs and cli_args.rs.

Acceptance criteria:
- #1: baseline and final CPU (native and maximized, playing and stopped), the frame time (eframe cpu_usage via ORB_UI_FRAME_STATS, headless show/tessellate times) and the per-frame counts (559 shapes, 524 meshes, 14112 vertices, 112 textures native) are in the notes with the scripts and commands.
- #2: playing 55% (native) and 50% (maximized) of the baseline CPU in paired runs (target half or less: met maximized, 5 points short native); stopped with nothing moving: no frame is drawn (0.18-0.20 s CPU in 20 s, against 4.6-5.1 s), tests/repaint.rs.
- #3: the four --screenshot pairs (two sizes, two densities: panel scales 1, 1.44, 2, 2.88) are byte-identical. Following playback: 28 frames/s while playing, each frame takes the snapshot as before; the step lights of a sixteenth (75 ms at 200 BPM) span two frames or more. The by-eye check of the new build was not possible (the screen was locked from 16:20): the supervisor or the user should watch the playing panel once.
- #4: cargo +nightly fmt --all --check clean, cargo clippy --all-targets -D warnings clean, debug cargo test -p orb-ui all pass, cargo test --release -q --no-fail-fast 669 passed, 0 failed, 3 ignored (70 suites); docs/ui_spec.md section 16 states the repaint rule ('Repainting (task-153)'); docs/building.md lists --play and ORB_UI_FRAME_STATS.

Review round 1 (send back), fixed:
1. Stale part textures after a resize (blocking). The map kept egui::load::SizedTexture (only an id) while egui's texture loader, bypassed on a hit, evicted the unused sizes of a URI (egui-0.37.0/src/load/texture_loader.rs:119-135, end_pass), freeing them; meshes then used dead ids. Reproduced with the new test against the first-round app.rs: 242 of 532 meshes used a freed texture after 624x510 -> 700x600. Fix: paint_part_with rasterizes a missing part itself as egui_extras' SVG loader does (egui_extras::image::load_svg_bytes_with_size with resvg::usvg::Options::default(), as SvgLoader::default without the svg_text feature; then ctx.load_texture(uri, image, LINEAR), as the default texture loader) and the map owns the TextureHandle. No loader is involved any more, so nothing else frees these textures. tests/part_textures.rs: resize 624x510 -> 700x600 -> back, twice; no mesh draws a texture outside egui's live set (set before the draw, free after it); 0 now, 242 on the first-round code.
2. Eviction: PartTextures drops a texture not drawn for PART_TEXTURE_FRAMES = 120 frames (swept once per 120 passes, by ctx.cumulative_pass_nr). The wrong comment is gone. Test: after 2*120+2 frames back at the first size, the map holds exactly the first size's count again.
3. AC #3 rewritten to what was verified: identical screenshots, about 28 frames/s while playing, and the new headless test the_panel_follows_a_playing_song_frame_by_frame (Trakka Rekka, a frame every 33 ms of audio for 3 s: position never goes back and moves, the 909 running light changes >= 20 times, meters lit in > 60 of 90 frames, every frame asks for a repaint within METER_FRAME). The by-eye check of the playing panel is deferred to a supervisor follow-up (created on main after the merge).
4. Stopped with audio: orb_player::output::Health::started() is already public (output.rs:146), so no orb-player change was needed. New test a_stopped_sounding_panel_asks_for_no_repaint_and_a_playing_one_does: App with audio_health Some(Health::started()), stopped: no repaint (> 60 s); after Play: METER_FRAME on each frame.
5. tests/frame_cost.rs: upper bounds (shapes <= 680, meshes <= 650, vertices <= 17000 native / 20000 at 1800x1100 2x; measured 559, 524/534, 14112/16412), and 2 timed frames in a debug run instead of 200 (10 in all with the warm-up).
6. FrameStats::frame returns the line (the window prints it); unit test a_line_every_two_seconds_of_frames. --play stays covered by the parse tests only (it needs a window).
7. docs/ui_spec.md: the stopped 0.2 s is the whole process by ps; the PartTextures text says what owns and frees the textures. Final summary: native reached 55% of the baseline, so the half-or-less target is missed there; maximized reached 50%.

After the fixes: paired, native 624x510, 17:07 (load average 4-10): base 19.2 / 18.5%, round-1 build 9.9 / 10.0%, this build 10.1 / 10.2% (54%). Screenshots base vs this build: the four pairs byte-identical. fmt and clippy -D warnings clean; debug cargo test -p orb-ui 253 passed in 20 suites; cargo test --release -q --no-fail-fast 674 passed, 0 failed, 3 ignored (71 suites).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The playing panel uses about half the CPU it did, and a stopped panel draws nothing.

Branch worktree-agent-a00a2a667598a3060, one commit on main 74cbe68 (amended for review round 1).

Against the half-or-less target:
- Native size reached 55% of the baseline, so the target is missed there.
- Maximized reached 50%, so the target is met there.

What changed (crates/orb-ui):
- Repaint rule. link.rs Link::moving, app.rs METER_FRAME (33 ms), model.rs Panel::next_change. The window repaints at about 28 frames/s while the song plays, a meter is lit, the gain-reduction LEDs change, or commands are not yet in the snapshot. A 303 lit note asks for a repaint at its end. Otherwise the window asks for no frame. Before, it asked for one every 16 ms whenever a player link existed.
- Per-frame waste removed, with the same picture:
  - Part textures are rasterized once per (part, pixel size), the way egui_extras does it, and kept with their handles in a map that frees a texture after 120 frames without a draw. Before, every call went through egui's loaders.
  - Knob arcs (three quarters of the vertices) are tessellated once per look.
- For measuring: the hidden --play option and ORB_UI_FRAME_STATS=1 (docs/building.md), and tests/frame_cost.rs.

Numbers (paired runs, Trakka Rekka, 20 s each):
- Playing, native 624x510: 20.7% -> 11.3% (55% of before; 54% after the review fixes).
- Playing, maximized: 22.6% -> 11.3% (50%).
- 57-58 -> 28 frames/s.
- Stopped: 4.6-5.1 s -> 0.2 s of whole-process CPU in 20 s, with no frames drawn.

Tried and dropped, both pixel-identical but with no CPU gain:
- Batching shapes by texture.
- An offscreen panel cache with dirty regions.

Screenshots (--screenshot, two sizes x two densities) are byte-identical to the baseline.

Tests:
- New: tests/repaint.rs (including the stopped-with-sound case and the panel following a playing song frame by frame), tests/part_textures.rs (resize and back, eviction), a FrameStats unit test, and bounds in frame_cost.
- fmt and clippy clean; debug orb-ui tests pass.
- cargo test --release: 674 passed, 0 failed.
- No engine change, so no corpus run.

Deferred: the by-eye check of the playing panel goes to a supervisor follow-up on main.
<!-- SECTION:FINAL_SUMMARY:END -->
