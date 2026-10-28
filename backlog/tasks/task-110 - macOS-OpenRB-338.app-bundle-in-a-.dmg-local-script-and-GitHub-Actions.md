---
id: TASK-110
title: 'macOS: OpenRB-338.app bundle in a .dmg (local script and GitHub Actions)'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 10:12'
updated_date: '2026-10-28 11:55'
labels:
  - tools
  - ui
milestone: m-4
dependencies:
  - TASK-108
  - TASK-104
references:
  - crates/rb-ui/src/art.rs
priority: medium
type: feature
ordinal: 130000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) for a macOS .dmg, built locally and by GitHub Actions. Double-clicking an unbundled binary in Finder opens Terminal, so the UI needs an .app bundle to launch like an app. A bundle also gives the Dock and Activity Monitor their name and icon properly (task-104 names the binary OpenRB-338 for now).

What:
- A script (scripts/, using task-108's build) makes OpenRB-338.app:
  - Contents/MacOS/OpenRB-338, a universal binary (aarch64 + x86_64 via lipo);
  - Contents/Info.plist: CFBundleName and CFBundleDisplayName OpenRB-338, a bundle identifier (propose one, e.g. io.github.pingw33n.openrb338), CFBundleShortVersionString from the workspace version, CFBundleVersion from the build, LSMinimumSystemVersion, NSHighResolutionCapable, and LSApplicationCategoryType music;
  - Contents/Resources/OpenRB-338.icns, generated from the app_icon part (rb_ui::art; render the iconset sizes 16..1024 with resvg, then iconutil).
- Include the CLI: put rb (later orb, task-068) in the .dmg next to the app, with a short note on putting it on PATH. Alternatively put it inside the bundle at Contents/MacOS/rb with a symlink hint. Choose one and say why.
- The .dmg: hdiutil (or create-dmg) with the app, an /Applications symlink for drag-install, a background or icon layout if cheap, and the license and readme. Name it OpenRB-338-<version>.dmg.
- release.yml builds the .dmg on a macos runner and uploads it.
- Signing: ad-hoc codesign (codesign -s -) so it runs locally. Developer ID signing and notarization need the user's Apple developer account and secrets. Leave hooks for them (env-gated steps that are skipped when the secrets are absent) and write down for the user what Gatekeeper shows for an unsigned download (right-click > Open, or xattr -d com.apple.quarantine).
- Check here: mount the .dmg, copy the app to a temp folder, launch it, and check ps and the Dock name; record in the notes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A local script produces OpenRB-338-<version>.dmg with a universal OpenRB-338.app (Info.plist, .icns from app_icon) and the CLI; the app launches from the mounted image
- [ ] #2 docs/building.md describes it; validated with actionlint (or act)
- [ ] #3 release.yml builds and uploads the .dmg on a macos runner; no signing or notarization (ad-hoc codesign only where launch needs it); the unsigned-app behavior is documented
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
User decisions (2026-10-28): no signing or notarization at all: drop the signing hooks, keep only ad-hoc codesign if the app needs it to launch on Apple Silicon (it does for arm64 binaries), and document Gatekeeper's right-click > Open. Both architectures: the universal app covers amd64 and arm64 in one .dmg; if a universal binary is impractical, make two .dmgs (-arm64, -x86_64).

The user (2026-10-28): with the unbundled binary (task-104) the Dock tooltip and Activity Monitor name show OpenRB-338, but Activity Monitor shows the generic executable (terminal) icon: it takes the icon from the bundle (CFBundleIconFile), which an unbundled binary does not have; the Dock icon comes from eframe's runtime icon. Check in this task that Activity Monitor shows the app icon for the bundled app, and also provide a dev script (e.g. scripts/macos-dev-app.sh) that wraps target/release/OpenRB-338 into a local OpenRB-338.app for everyday runs, so the icon is right without building the .dmg.
<!-- SECTION:NOTES:END -->
