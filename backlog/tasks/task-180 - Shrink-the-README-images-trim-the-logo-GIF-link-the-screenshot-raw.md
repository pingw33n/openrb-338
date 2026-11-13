---
id: TASK-180
title: Shrink the README images, trim the logo GIF, link the screenshot raw
status: Done
assignee:
  - '@agent-worker'
created_date: 2026-11-13 14:10
updated_date: '2026-11-13T14:17:00Z'
labels:
  - docs
  - ui
milestone: m-3
dependencies: []
references:
  - README.md
  - assets/README.md
  - assets/readme/logo.gif
  - assets/readme/screenshot.png
  - task-179
priority: medium
type: docs
ordinal: 196000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Why
task-179 (the panel redesign, commit cecaf28) regenerated the README's images and the art's previews from the app. The new speckled panel texture compresses badly, so the images grew a lot:
- `assets/readme/screenshot.png`: 1.7 MB to 4.6 MB.
- `assets/readme/logo.gif`: 606 KB to 2.3 MB (740x216, 56 frames).
- `docs/ui_app/panel_*.png`: up to 3.8 MB each.
- `assets/ui/preview/*.png`: up to 5 MB each.

The user (@pingw33n) decided on 2026-11-13:
1. **Every committed PNG and GIF of the README, docs/ui_app and assets/ui/preview:** recompress losslessly, with `oxipng -o max --strip all` and `gifsicle -O3`, so the pixels stay identical.
2. **Only `assets/readme/screenshot.png` and `assets/readme/logo.gif`, the two images a README visitor downloads:** a lossy color reduction too.
   - Use `pngquant` (quality around 80-95) for the PNG.
   - For the GIF, use `gifsicle --colors` or `--lossy`. Fewer frames are allowed only if the animation stays smooth.
   - Keep the resolution unchanged.
3. **Keep lossless:** `docs/ui_app/*.png` and `assets/ui/preview/*.png`. They are used to check the art pixel by pixel.
4. **Trim the logo GIF.** Its bottom rows show the tops of the subtitle's letters ("BASS LINE AND DRUM MACHINE" or similar, cut off). Crop it so that no frame has any pixel of that text, and keep the plate with its shadow whole. Check every frame, not only the first; the animation may move or reveal the text in later frames.
5. **README.md's "Screenshot" link** (line 13, now `assets/readme/screenshot.png`) must open the raw image, not GitHub's file page. Use `https://raw.githubusercontent.com/pingw33n/OpenRB-338/main/assets/readme/screenshot.png`.

## Tools
oxipng and gifsicle are in /opt/homebrew/bin. The user says pngquant is installed too, but `which pngquant` did not find it on 2026-11-13. Look under /opt/homebrew/bin and `brew list`. If it is really missing, stop and report; do not install anything.

## Reproducibility
The task-179 agent made logo.gif and screenshot.png with a scratch script that is not in the repository. If you find how they are made (git grep, the task-179 notes, `ORB_LOGO_T`, `--size` options), write the steps, including the new crop and compression, into assets/README.md where it lists these files, so that the next regeneration repeats them. If the generator is not reproducible from the repository, say so in the notes and document only the compression and crop commands.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every PNG under assets/readme, docs/ui_app and assets/ui/preview is run through oxipng: docs/ui_app and preview files decode to exactly the same pixels as before (checked by a script that compares decoded RGBA), with the sizes before and after in the notes
- [x] #2 assets/readme/screenshot.png is color-reduced at its current resolution and looks the same at 100% zoom (the notes name the quality and the size before/after)
- [x] #3 assets/readme/logo.gif is cropped so no frame contains any pixel of the subtitle text below the plate (checked on every frame, crop box in the notes), keeps the plate and its shadow whole, plays as before, and is optimized with gifsicle (size before/after in the notes)
- [x] #4 README.md's Screenshot link points at https://raw.githubusercontent.com/pingw33n/OpenRB-338/main/assets/readme/screenshot.png
- [x] #5 assets/README.md records the crop and compression commands for the next regeneration
- [x] #6 No other files change except the task file; one commit
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Crop logo.gif row 215 (subtitle tops), gifsicle -O3 --colors 96 --lossy=30; oxipng all PNGs; pngquant 80-95 screenshot; raw link; document in assets/README.md.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: task-180-readme-images

Tools: pngquant, oxipng, gifsicle in /opt/homebrew/bin. Logo GIF: all 56 frames scanned (PIL, per-row max luminance): only row 215 (of 0..215) has text pixels (lum 181, in every frame); rows 202-214 are background/shadow fade (lum 30), plate and shadow whole. Crop box 0,0+740x215. Left side: 36 px empty margin before the icon, right margin 15; kept (deliberate padding, little to save). GIF: 2329951 -> 835077 B (gifsicle -O3 --colors 96 --lossy=30 --crop 0,0+740x215; 56 frames, 30 ms delay kept; lossless -O3 alone gave 2318296; colors 128/lossy 30 gave 991733, 64 gave 629299). Screenshot: pngquant --quality=80-95 --speed 1 then oxipng: 4611413 -> 1105498 B, same size 2200x1444, mean abs diff 0.98/255, max 32; looks identical at 100%. Lossless oxipng -o max --strip all, decoded RGBA identical (script compare): preview panel_1x 459750->397976, 2x 1505975->1300913, 3x 2978806->2585808, parts_1x 785090->605412, 2x 2503419->1889980, 3x 5087399->3895195; docs/ui_app 1000x600 3768429->2629972, 312x255 663457->419008, 624x510 2128687->1322552. Generator of logo.gif not in the repo: documented only crop and compression.

Review round 1: (1) My first shadow check used per-row MAX luminance, which hid the gradient; the --colors 96 encode flattened the shadow (frame 55, col 370 rows 203,205..213 orig 12,15,18,22,24,26 vs new 26,27,26,27,26,26; worst diff 24 over rows 190-214). A shared palette (--colors 256 even, no lossy) still gave 16,16,16,21,24,26 (worst 9); --colors 128 --dither 25,25,...; local palettes kept (no --colors) do: --lossy=30 gives frame 0 orig 13,15,18,23,25,26 vs 13,15,19,26,24,26; frame 27 and 55 orig 12,15,18,22,24,26 vs 12,16,18,22,24,27; worst diff 14, mean 0.58 over rows 190-214 of all 56 frames (sampled every 3rd column). --lossy=80 and 200 were worse (worst 15/17, mean 1.1/1.3), lossy 0 is exact at 2318296. New flags: gifsicle -O3 --lossy=30 --crop 0,0+740x215 (local palettes kept), size 2040085 B (was 835077 with --colors 96; original 2329951), 56 frames. (2) The pngquant command that made the file was --quality=80-95 --strip --speed 1 (as in the doc; my notes omitted --strip); notes now say so. (3) assets/README.md line wrapped to <= 96 columns. (4) gifsicle command in assets/README.md updated.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Branch task-180-readme-images. Screenshot 4.6 MB -> 1.1 MB (pngquant 80-95 --strip --speed 1 + oxipng), logo.gif 2.3 MB -> 2.04 MB, cropped to 740x215 (no subtitle pixels in any frame) with the drop shadow kept (gifsicle -O3 --lossy=30, local palettes), the 9 other PNGs losslessly smaller with identical pixels, Screenshot link is the raw URL, assets/README.md records the commands. Docs only: no tests.
<!-- SECTION:FINAL_SUMMARY:END -->
