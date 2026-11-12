---
id: TASK-177
title: >-
  OpenRB-338 on Windows uses DirectX 12 by default (Intel Vulkan driver crash at
  startup)
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-11-12T07:30:00Z'
updated_date: '2026-11-12T07:51:00Z'
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
- [x] #1 On Windows, OpenRB-338 creates its wgpu instance with DX12 only unless WGPU_BACKEND is set, and WGPU_BACKEND still overrides; macOS and Linux keep Backends::PRIMARY | GL; a unit test covers the choice
- [x] #2 On the Windows test machine ORB_GPU_INFO=1 prints a Dx12 adapter with nothing set and a Vulkan adapter with WGPU_BACKEND=vulkan (outputs in the notes); the no-DX12-adapter path's message is recorded in the notes
- [x] #3 docs/building.md states the Windows default and the override with the reason (issue #10); the Windows notes give the WGPU_BACKEND=dx12 workaround for v2026.1103.0917; fmt, clippy and cargo test --release pass
- [x] #4 The task notes hold a reply text for issue #10
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. main.rs: fn wgpu_backends(env, windows) + wgpu_setup() building WgpuSetup::CreateNew(without_display_handle()) with backends overridden; unit test.
2. docs/building.md backend paragraph + README Windows note.
3. Build orb-ui for Windows (cargo xwin or on the machine), run with ORB_GPU_INFO=1 on the test machine under permits.
4. fmt, clippy, cargo test --release; notes, ACs, commit.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: task-177-windows-dx12

Code: crates/orb-ui/src/main.rs. wgpu_backends(from_env, windows) picks Backends::from_env() else DX12 on Windows, else PRIMARY|GL; wgpu_setup() builds eframe::egui_wgpu::WgpuSetup::CreateNew(WgpuSetupCreateNew::without_display_handle()) and overrides only instance_descriptor.backends (egui-wgpu 0.37 src/setup.rs:245-246 default; power preference, flags, device descriptor untouched). Unit tests: backends_default_is_dx12_on_windows_and_the_env_wins, a_wgpu_start_error_names_wgpu_backend. eframe re-exports egui_wgpu and wgpu (eframe lib.rs:162), no new dependency.
No-DX12 path: eframe's error was only a Result returned from main (stderr: 'Error: Wgpu(CreateSurfaceError(... FailedToCreateSurfaceForAnyBackend({})))'), and a release Windows build is a GUI-subsystem exe with no console, so the user saw nothing, and the text did not name WGPU_BACKEND. Added report_start_error/start_error_text in main.rs: stderr line plus an rfd message box (crash::no_dialog, now pub, turns the box off) saying 'OpenRB-338 could not open its window: ...' and 'The WGPU_BACKEND environment variable picks the graphics backend: vulkan, gl or dx12 (Windows uses dx12 by default).'. The box itself was not run on the machine (ORB_UI_NO_CRASH_DIALOG=1 for the runs, a modal box would block); the text was.
Windows test machine (permits: all 6 of redc-win-run held as task-177-windows-dx12 and released). Built with cargo xwin x86_64-pc-windows-msvc, CL=-Wno-incompatible-pointer-types (scripts/build.sh), plus CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true so the exe is a console program: the shipped release exe (windows_subsystem) lost stderr under the rcs exec redirect (even --help printed nothing). Run through a .bat in C:\re\rb\t177 with ORB_GPU_INFO=1 and ORB_UI_NO_CRASH_DIALOG=1, stdout+stderr to a file, killed with taskkill after 10-30 s (rc=1 is the taskkill):
 nothing set: 'gpu: Dx12 IntegratedGpu AMD Radeon (TM) Graphics' (DX12 init takes several seconds; a 10 s first run printed no gpu line yet)
 WGPU_BACKEND=dx12: gpu: Dx12 IntegratedGpu AMD Radeon (TM) Graphics
 WGPU_BACKEND=vulkan: gpu: Vulkan IntegratedGpu AMD Radeon (TM) Graphics
 WGPU_BACKEND=gl: gpu: Gl Other AMD Radeon (TM) Graphics
 WGPU_BACKEND=vulkan,dx12: gpu: Vulkan ...
 WGPU_BACKEND=metal (no adapter for any backend): 'OpenRB-338 could not open its window: WGPU error: Failed to create surface for any enabled backend: {}' + the WGPU_BACKEND paragraph above.
The Intel igvk64.dll crash cannot be reproduced: the machine has an AMD GPU and no Intel Vulkan driver. The fix rests on the issue's report that WGPU_BACKEND=dx12 starts, and on wordcraft#170.
Tests: cargo +nightly fmt --all --check clean; cargo clippy --all-targets -- -D warnings clean; cargo test --release all pass (macOS host).
Reply text for issue #10 (post nothing): 'Thanks for the report and the Event Viewer details. The crash is in Intel's Vulkan driver (igvk64.dll): OpenRB-338 asked wgpu for every graphics backend, so the driver was loaded and enumerated even though DirectX 12 would have been used. The next release asks for DirectX 12 only on Windows (it can still be changed with the WGPU_BACKEND environment variable, vulkan or gl), so the Vulkan driver is never loaded. Until then, as you found, start v2026.1103.0917 with WGPU_BACKEND=dx12: in cmd, run set WGPU_BACKEND=dx12 and then OpenRB-338.exe, or set it permanently in the Windows environment variables. I could not reproduce the crash itself (no Intel Vulkan driver here), so please say if the new release still fails.'

Review round 1: (1) commit subject area changed to ui (user correction, supersedes the first instruction to use tools); (2) start_error_text now adds the WGPU_BACKEND paragraph only for WgpuError::RequestAdapterError and CreateSurfaceError (no adapter / no surface), other wgpu errors get only eframe's text; test only_a_no_adapter_error_names_wgpu_backend covers both; (3) after report_start_error main calls std::process::exit(1), so the Rust runtime does not print a second 'Error:' line. Backend choice unchanged, so the Windows machine was not used again. Re-ran cargo test --release -p orb-ui, clippy -D warnings, nightly fmt --check.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Windows OpenRB-338 now creates its wgpu instance with DX12 only unless WGPU_BACKEND is set (crates/orb-ui/src/main.rs wgpu_backends/wgpu_setup, unit-tested); macOS and Linux keep PRIMARY|GL. A window failure (no adapter) now shows a box and a stderr line naming WGPU_BACKEND. Verified on the Windows test machine: nothing set -> Dx12, WGPU_BACKEND=vulkan -> Vulkan, gl -> Gl, metal -> the new message. docs/building.md and README updated. The Intel crash itself is not reproducible here. Tests: fmt, clippy, cargo test --release pass. Branch task-177-windows-dx12, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->
