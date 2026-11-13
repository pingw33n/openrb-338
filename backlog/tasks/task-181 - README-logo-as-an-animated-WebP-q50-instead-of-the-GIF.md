---
id: TASK-181
title: README logo as an animated WebP (q50) instead of the GIF
status: Done
assignee:
  - '@agent-worker'
created_date: 2026-11-13 14:24
updated_date: '2026-11-13T14:26:00Z'
labels:
  - docs
  - ui
milestone: m-3
dependencies:
  - TASK-180
references:
  - README.md
  - assets/README.md
  - assets/readme/logo.gif
  - task-180
priority: medium
type: docs
ordinal: 197000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Why
task-180 compressed the README's logo animation, assets/readme/logo.gif, to 2.04 MB. The panel's fine speckle defeats GIF palettes, and the plate's drop shadow needs per-frame palettes, so the GIF cannot get much smaller.

The supervisor compared animated WebP encodes, made with libwebp's gif2webp from the full-quality GIF (`git show 17a224b:assets/readme/logo.gif`, 2,329,951 B), with its subtitle row cropped:

| Encode | Size |
|---|---|
| lossless | 1.91 MB |
| lossy q85 | 1.23 MB |
| lossy q75 | 0.75 MB |
| lossy q50 | 0.52 MB |

On 2026-11-13 the user (@pingw33n) chose lossy q50 (mean difference 4.6 out of 255; it mostly softens the speckle).

## What to do
1. **Make `assets/readme/logo.webp` from the full-quality GIF**, not from the GIF now on main, which is already lossy.
   - `git show 17a224b:assets/readme/logo.gif > orig.gif`
   - `gifsicle --crop 0,0+740x215 orig.gif -o crop.gif` (the crop task-180 chose; row 215 holds subtitle text)
   - `gif2webp -lossy -q 50 -m 6 crop.gif -o assets/readme/logo.webp` (`-m 4` gave 521,976 B, and `-m 6` may be smaller)

   The tools are in /opt/homebrew/bin. Keep all 56 frames, the 30 ms frame delay and the looping.
2. **Delete `assets/readme/logo.gif`** with `git rm`.
3. **Point README.md line 2 at `assets/readme/logo.webp`**, keeping `width="378"` and the alt text. GitHub renders animated WebP in a README.
4. **Update assets/README.md:** the file list (line 9 describes logo.gif) and the regeneration commands that task-180 added (the gifsicle line becomes the crop plus gif2webp).
5. **Check nothing else names logo.gif:** run `git grep logo.gif`. Leave the backlog task files alone; they are history.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 assets/readme/logo.webp exists: animated, 740x215, 56 frames at 30 ms, looping, lossy q50 from the full-quality GIF of 17a224b with the 0,0+740x215 crop; size in the notes
- [x] #2 assets/readme/logo.gif is removed and nothing outside backlog/ refers to it (git grep)
- [x] #3 README.md shows logo.webp at width 378 with the same alt text
- [x] #4 assets/README.md describes logo.webp and records the commands that make it
- [x] #5 One commit; only README.md, assets/README.md, assets/readme/logo.{gif,webp} and the task file change
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Build crop.gif from 17a224b with gifsicle, encode with gif2webp -lossy -q 50 -m 6. 2. git rm logo.gif, repoint README.md. 3. Update assets/README.md. 4. Verify with Pillow/webpmux, git grep.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: task-181-logo-webp (rebased on main e7b2d6e)

logo.webp: gif2webp -lossy -q 50 -m 6 from the 17a224b GIF cropped 0,0+740x215 = 484,304 B (-m 4 gave 521,976 B, as the task says). Verified: Pillow 740x215, 56 frames; webpmux -info shows every frame at 30 ms, Loop Count 1. The source GIF has no loop extension (plays once), so loop count 1 keeps its behavior. The search for the old file name outside backlog/ finds nothing.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
assets/readme/logo.webp (484,304 B, lossy q50, -m 6, 56 frames, 30 ms, 740x215, loop count 1 as in the source GIF) replaces the 2.04 MB GIF; README.md and assets/README.md updated. Branch task-181-logo-webp, one commit. Tests: none (docs and asset).
<!-- SECTION:FINAL_SUMMARY:END -->
