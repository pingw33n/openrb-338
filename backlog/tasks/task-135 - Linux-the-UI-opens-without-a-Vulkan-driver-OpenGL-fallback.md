---
id: TASK-135
title: 'Linux: the UI opens without a Vulkan driver (OpenGL fallback)'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-29 06:03'
labels:
  - ui
  - tools
milestone: m-4
dependencies:
  - TASK-112
references:
  - crates/rb-ui/src/main.rs
  - scripts/appimage-smoke.sh
  - docs/building.md
priority: medium
type: bug
ordinal: 154000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the task-112 smoke runs (2026-10-29, ubuntu:22.04 container under xvfb, aarch64) found that OpenRB-338 gets a window only with Vulkan. WGPU_BACKEND=vulkan (lavapipe) worked. WGPU_BACKEND=gl, and a run with the Vulkan ICDs hidden, both failed with FailedToCreateSurfaceForAnyBackend. A Linux user without a Vulkan driver (older GPUs, some VMs, remote X) would get no window. docs/building.md now says a Vulkan driver is required.

What:
1. Find out why wgpu's GL backend can't create a surface. It could be xvfb's missing GLX or EGL in the container, or eframe/wgpu's GL path on X11 needing features we don't enable. Check with mesa's llvmpipe and EGL in the container, and with a real X11 display if possible.
2. If wgpu GL can work, make it work: enable the needed wgpu or eframe features and set the backend order so that it falls back from Vulkan to GL automatically.
3. If it can't, consider eframe's glow renderer as a fallback. The UI uses egui_extras SVG textures; check the texture-size cap of task-119 under glow.
4. Keep macOS (Metal) and Windows (DX12/Vulkan) unchanged. Measure any size change of the binaries.
5. Update appimage-smoke.sh to also take a screenshot with Vulkan hidden, so the fallback is tested in CI, and update docs/building.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 OpenRB-338 opens and screenshots on Linux with no Vulkan ICD (GL fallback), shown in the ubuntu:22.04 smoke test; Vulkan still preferred when present
- [ ] #2 macOS and Windows rendering unchanged; size change measured; docs updated; fmt, clippy, cargo test --release, shellcheck and actionlint pass
<!-- AC:END -->
