---
id: TASK-135
title: 'Linux: the UI opens without a Vulkan driver (OpenGL fallback)'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-29 06:03'
updated_date: '2026-10-29 07:07'
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
- [x] #1 OpenRB-338 opens and screenshots on Linux with no Vulkan ICD (GL fallback), shown in the ubuntu:22.04 smoke test; Vulkan still preferred when present
- [x] #2 macOS and Windows rendering unchanged; size change measured; docs updated; fmt, clippy, cargo test --release, shellcheck and actionlint pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Build the aarch64 binaries in the Docker image (scripts/build.sh). 2. In ubuntu:22.04 under xvfb, try WGPU_BACKEND=gl with libegl1, libgl1-mesa-dri, libglx-mesa0 installed, to find why the surface failed (suspect missing libEGL: wgpu-hal gles loads libEGL dynamically; wgpu default features already include gles). 3. If gl works with the right host libs: set the backend order Vulkan then GL explicitly for Linux only, check the texture cap under GL (task-119). 4. If libs only: document and add them to smoke; else consider eframe glow. 5. Update appimage-smoke.sh (second screenshot with Vulkan hidden), docs/building.md; macOS check by --screenshot before and after; fmt, clippy, tests, shellcheck, actionlint.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-af44f8ab5f6990076

Cause found (aarch64, ubuntu:22.04, xvfb, binary built by scripts/build.sh, dist profile): the GL backend failed only because libEGL.so.1 was not installed (wgpu loads it with dlopen). Runs: no libegl1, WGPU_BACKEND=gl: Error: Wgpu(CreateSurfaceError(... FailedToCreateSurfaceForAnyBackend({}))), no PNG. With libegl1 (xvfb already pulls Mesa libegl-mesa0, libgl1-mesa-dri, libglx-mesa0): WGPU_BACKEND=gl PNG 191621 B; no Vulkan ICD and backend automatic PNG 191621 B (same size as the Vulkan one). A --size 3000x2400 --ppp 2 window under GL: PNG 435604 B, no panic (task-119 cap OK under GL). eframe already enables wgpu gles (wgpu default features) and egui-wgpu defaults to Backends PRIMARY|GL, so Vulkan is tried first and the fallback is automatic; the glow renderer is not needed. No Rust or Cargo change, so no size change: AppImage aarch64 14547464 B before and after (same as task-112). Source reading: wgpu-core instance.rs get_order: DiscreteGpu 1, IntegratedGpu 2, Other (what GL reports) 3, Cpu 5, so a hardware Vulkan GPU wins over GL, but a software GL (llvmpipe) is picked before lavapipe; not run, source only. The change: scripts/appimage-smoke.sh (libegl1; 3 screenshots; shot helper) and docs/building.md. Smoke on the rebuilt aarch64 AppImage: SMOKE-OK with three PNG signatures. Not run: x86_64 (moved to task-134 AC), a real desktop. macOS and Windows: the diff has no Rust, Cargo or Cargo.lock change (git diff --stat: docs/building.md, scripts/appimage-smoke.sh), so the binaries are the same; a before screenshot on macOS was taken (mac_before.png, 1003465 B) but an after one would compare identical code, so not repeated. Checks: fmt, clippy, 586 release tests, 54+... rb-ui tests, shellcheck, actionlint clean. Docker: no container of mine left; no new images (the build and ubuntu:22.04 images were reused).

Review round 1 (supervisor, send back), done: (1) CORRECTION of my earlier note and docs on the adapter order: llvmpipe (GL) and lavapipe (Vulkan) are both DeviceType::Cpu, wgpu-core sorts stably by get_order, so the backend order decides and Vulkan wins; GL reports IntegratedGpu on Intel or Qualcomm, not other. docs/building.md reworded. (2) The mechanism is PRIMARY|GL plus PowerPreference::HighPerformance and the device-type sort; docs reworded. (3) Shown by a run: rb-ui now prints the chosen adapter when OPENRB_GPU_INFO=1 (crates/rb-ui/src/main.rs, 7 lines, reads cc.wgpu_render_state.adapter.get_info()). appimage-smoke.sh on the rebuilt aarch64 AppImage (libegl1, lavapipe present, ICDs not hidden, nothing forced): 'gpu: Vulkan Cpu llvmpipe (LLVM 15.0.7, 128 bits)'; WGPU_BACKEND=vulkan: Vulkan; WGPU_BACKEND=gl: Gl; Vulkan ICDs hidden: 'gpu: Gl Cpu llvmpipe'; each PNG 191621 B; the script asserts the backend of all four (SMOKE-OK). So AC1 holds: Vulkan is chosen when present, GL when not. (4) cargo test -p rb-ui (debug) 197 passed 0 failed across its test binaries; workspace cargo test --release 586 passed 0 failed. (5) docs nits fixed. Code change means the earlier 'no Rust change' is wrong: macOS: OPENRB_GPU_INFO=1 --demo --screenshot printed 'gpu: Metal IntegratedGpu Apple M1 Pro' and the PNG is byte-identical to the one before (cmp: SAME); the print is only when the variable is 1. Windows: cargo xwin clippy --release -p rb-ui --target x86_64-pc-windows-msvc -D warnings clean (with CL=-Wno-incompatible-pointer-types as build.sh sets). Sizes: aarch64 dist OpenRB-338 20528872 -> 20537064 B (+8192); the aarch64 AppImage is 14547464 B before and after (compressed, same size).

Review round 2: the AppImage size claim holds. ls -l: dist/aarch64-unknown-linux-gnu/bin/OpenRB-338 20537064 B (built 08:50:50, +8192 B over the 20528872 B before); dist/aarch64-unknown-linux-gnu/OpenRB-338-2026.1028.1930-aarch64.AppImage 14547464 B, packed at 08:59:46, after the binary, and the same size as the AppImage from the first build. The 8 KB are absorbed by squashfs compression and block padding (the explanation is the reviewer's; I did not diff the squashfs contents).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
No Vulkan driver is needed: the GL fallback already worked; the missing piece was libEGL.so.1 (package libegl1) on the host, which wgpu loads at run time. In ubuntu:22.04 (aarch64, xvfb) the OpenGL and the Vulkan-hidden runs wrote a PNG once libegl1 was there and failed with FailedToCreateSurfaceForAnyBackend without it. egui-wgpu asks for Backends PRIMARY|GL with PowerPreference HighPerformance; wgpu sorts the adapters by device type and a tie (lavapipe and llvmpipe are both Cpu) keeps the backend order, Vulkan first, so Vulkan is chosen when present: shown by a run, OPENRB_GPU_INFO=1 prints 'gpu: Vulkan Cpu llvmpipe' (automatic) and 'gpu: Gl ...' (ICDs hidden). Changes: scripts/appimage-smoke.sh (libegl1; four screenshots, each asserting the backend), crates/rb-ui/src/main.rs (the OPENRB_GPU_INFO=1 print, 7 lines), docs/building.md. macOS rendering: PNG byte-identical (cmp); Windows: xwin clippy clean. Binary +8192 B, aarch64 AppImage 14547464 B unchanged. The x86_64 smoke and a real desktop without Vulkan are a criterion of task-134. Checks: fmt, clippy, cargo test --release 586 passed, cargo test -p rb-ui 197 passed, shellcheck, actionlint. Branch worktree-agent-af44f8ab5f6990076, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->
