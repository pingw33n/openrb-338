---
id: TASK-058
title: Test RBM support on the user's third-party mods and songs (~/rb_stuff)
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-27 07:44'
labels:
  - rbm
  - tests
milestone: m-2
dependencies:
  - TASK-033
references:
  - docs/rbm.md
priority: high
type: task
ordinal: 33500
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The user's collection of third-party mods and songs is in ~/rb_stuff on this Mac (added 2026-10-27): ~/rb_stuff/Mods (172 .rbm) and ~/rb_stuff/songs (about 208 .rbs, some of them "... copy" duplicates; dedupe by SHA-256). These files are third-party and the user's: read them in place, never copy them into the repository; tests that need them read a path from an environment variable (e.g. RB_THIRDPARTY_DIR) and skip with a message when it is unset.

The m-2 work (tasks 030-033) was validated on four shipped mods only (PBE2, MSM2, Metallicon+, Orbit2). This task tests the RBM support against the whole collection and fixes what breaks:
1. Every .rbm: crates/rbm parses it, reads its key pages from RB20FUL.DAT (RB_RB20FUL), descrambles every embedded file, and loads the 46 samples (with per-slot fallback). Compare each mod's embedded files and samples with tools/rbm.py's extraction. List every failure with its cause (container variants: other CAT types, INFO layout or version differences, chunk order, sample formats such as other bit depths, sample rates, stereo, AIFC, short COMM chunks; names outside the 46; duplicate names). Fix rb where the original would load the mod (check the original's behavior in the disassembly with addresses, or on the test machine), and record genuinely broken files as such.
2. Every .rbs: crates/rbs reads it and writes it back (byte-identical for canonical files); `rb render` with --mod-dir ~/rb_stuff/Mods resolves its mod by name (list songs whose mod is missing from the collection or whose name matches only case-insensitively) and renders without a panic or fault.
3. A reference check: render a sample of at least 20 songs covering at least 10 different third-party mods with rbhost on the test machine (--mod, --rb20ful, --idle-blocks 9088; one redc-win-run permit per run; put files only under C:\re\rb\<folder> and clean up after) and show `rb render` is byte-exact against them; and cross-check 3 of them against the original app's export (tools/rb_export.py + tools/rbhost/fit.py, one app session with all permits taken only when free; the user sometimes runs ReBirth there — wait if it is running).
4. Write the results to docs/rbm.md (a section on the third-party collection: counts, failure classes, what rb does with each) — mod names and counts only, no content. A new defect of the original goes into docs/original_bugs.md.
Add a reproducible check: a tool or test (e.g. `tools/rbm.py check-dir` or an env-gated cargo test) that runs steps 1 and 2 over a directory and prints a summary.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every .rbm in ~/rb_stuff/Mods is checked: parse, key pages, descrambling and the 46 samples, compared with tools/rbm.py; failures are listed with causes and fixed where the original would load the mod
- [ ] #2 Every .rbs in ~/rb_stuff/songs round-trips through crates/rbs and renders with rb render (mod resolved or reported missing) without a panic
- [ ] #3 At least 20 songs over at least 10 third-party mods match rbhost byte for byte, and 3 match the original app's export at their fitted N
- [ ] #4 A reproducible directory check exists and docs/rbm.md records the results; no third-party file is committed
<!-- AC:END -->
