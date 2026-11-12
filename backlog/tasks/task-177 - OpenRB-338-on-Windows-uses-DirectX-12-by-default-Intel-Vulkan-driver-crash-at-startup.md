---
id: TASK-177
title: >-
  OpenRB-338 on Windows uses DirectX 12 by default (Intel Vulkan driver crash at
  startup)
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-11-12T07:30:00Z'
labels:
  - ui
  - docs
milestone: m-5
dependencies: []
references:
  - crates/orb-ui/src/main.rs
  - docs/building.md
  - https://github.com/pingw33n/openrb-338/issues/10
  - https://github.com/storytold/wordcraft/issues/170
priority: high
type: bug
ordinal: 193000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: GitHub issue #10 (2026-11-12). On a Windows laptop with an Intel GPU, OpenRB-338 v2026.1103.0917 exits silently before the window appears. Event Viewer: igvk64.dll (Intel's Vulkan driver) 31.0.101.2140, exception 0xc0000005, offset 0x64ea82. With WGPU_BACKEND=dx12 it starts fine. The same crash is reported for another egui/wgpu app, storytold/wordcraft#170 (Intel UHD 630, driver 31.0.101.1999): it happens while wgpu enumerates Vulkan physical devices; wordcraft fixed it by creating the wgpu instance with DX12 only on Windows, Vulkan only when WGPU_BACKEND asks for it.

Cause: crates/orb-ui/src/main.rs builds eframe::NativeOptions with the default wgpu_options. egui-wgpu 0.37 (src/setup.rs:245-246, WgpuSetupCreateNew::without_display_handle) asks for wgpu::Backends::from_env() or else Backends::PRIMARY | GL, which on Windows includes Vulkan, so the Vulkan driver is loaded and enumerated even when DX12 would be chosen. The crash is native (an access violation), so crash.rs's panic hook logs nothing and shows no box.

What (the user's decision, 2026-11-12: DX12 on Windows, no fallback probe):
- main.rs: set options.wgpu_options.wgpu_setup to WgpuSetup::CreateNew(WgpuSetupCreateNew::without_display_handle()) with instance_descriptor.backends = Backends::from_env().unwrap_or(DX12 on Windows, PRIMARY | GL elsewhere); everything else stays egui-wgpu's default (power preference, flags, device descriptor). Put the choice in a small function so a unit test covers it (WGPU_BACKEND unset gives DX12 on Windows and PRIMARY | GL elsewhere; set, it wins).
- No fallback: a Windows machine without a DX12 adapter gets eframe's error (no surface for any backend), which the user can override with WGPU_BACKEND=vulkan or gl. Check what OpenRB-338 shows then (WGPU_BACKEND forcing a backend with no adapter is one way to see the path) and make sure the message names WGPU_BACKEND; if eframe's error only goes to stderr, say so in the notes.
- macOS and Linux behavior must not change (Linux's Vulkan-then-OpenGL choice is in docs/building.md around line 720 and scripts/appimage-smoke.sh).
- docs/building.md: the backend paragraph covers Windows (DX12 by default, WGPU_BACKEND=vulkan|gl to override; why: the Intel Vulkan driver crash, issue #10). README's Windows notes (or wherever the Windows quick start is): one line on the silent exit of v2026.1103.0917 and the WGPU_BACKEND=dx12 workaround for that release.
- Verify on the Windows test machine (the agent brief, doc-001, has its access and permits): ORB_GPU_INFO=1 prints 'gpu: Dx12 ...' with nothing set, and WGPU_BACKEND=vulkan still selects Vulkan. The crash itself cannot be reproduced without that Intel driver; say so.
- A reply text for issue #10 (the fix, and the workaround until the next release) in the task notes; post nothing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 On Windows, OpenRB-338 creates its wgpu instance with DX12 only unless WGPU_BACKEND is set, and WGPU_BACKEND still overrides; macOS and Linux keep Backends::PRIMARY | GL; a unit test covers the choice
- [ ] #2 On the Windows test machine ORB_GPU_INFO=1 prints a Dx12 adapter with nothing set and a Vulkan adapter with WGPU_BACKEND=vulkan (outputs in the notes); the no-DX12-adapter path's message is recorded in the notes
- [ ] #3 docs/building.md states the Windows default and the override with the reason (issue #10); the Windows notes give the WGPU_BACKEND=dx12 workaround for v2026.1103.0917; fmt, clippy and cargo test --release pass
- [ ] #4 The task notes hold a reply text for issue #10
<!-- AC:END -->
