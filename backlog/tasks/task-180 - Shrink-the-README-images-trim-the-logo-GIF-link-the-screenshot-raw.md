---
id: TASK-180
title: 'Shrink the README images, trim the logo GIF, link the screenshot raw'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-11-13 14:10'
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
- [ ] #1 Every PNG under assets/readme, docs/ui_app and assets/ui/preview is run through oxipng: docs/ui_app and preview files decode to exactly the same pixels as before (checked by a script that compares decoded RGBA), with the sizes before and after in the notes
- [ ] #2 assets/readme/screenshot.png is color-reduced at its current resolution and looks the same at 100% zoom (the notes name the quality and the size before/after)
- [ ] #3 assets/readme/logo.gif is cropped so no frame contains any pixel of the subtitle text below the plate (checked on every frame, crop box in the notes), keeps the plate and its shadow whole, plays as before, and is optimized with gifsicle (size before/after in the notes)
- [ ] #4 README.md's Screenshot link points at https://raw.githubusercontent.com/pingw33n/OpenRB-338/main/assets/readme/screenshot.png
- [ ] #5 assets/README.md records the crop and compression commands for the next regeneration
- [ ] #6 No other files change except the task file; one commit
<!-- AC:END -->
