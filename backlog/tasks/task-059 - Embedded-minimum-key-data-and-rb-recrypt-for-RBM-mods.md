---
id: TASK-059
title: Embedded minimum key data and rb recrypt for RBM mods
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-27 07:44'
updated_date: '2026-10-27 07:44'
labels:
  - rbm
  - cli
milestone: m-2
dependencies:
  - TASK-033
references:
  - docs/rbm.md
priority: medium
type: feature
ordinal: 33700
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Decided by the user on 2026-10-27, superseding task-053's "nothing bundled" for this purpose: rb embeds the minimum amount of RB20FUL.DAT key data, and `rb recrypt` rewrites a .rbm so that it uses only that embedded data, so recrypted mods load without the user's CD. Mods that are not recrypted still need RB20FUL.DAT (task-053).

How (docs/rbm.md sections 5 and 6; discussed 2026-10-27): the two key pages are chosen only by sum[j] % (0x8000000 - 2048), where the sums start at {0x2000, 0x3000} and add the big-endian u32 words of the first 1024 bytes of every EMBF chunk (sum[0] the first 512 bytes, sum[1] the second), and those first 1024 bytes are never scrambled. Descrambling is buf[n] += table[(key&1)*2048 + n % P] for n past 1024, with key = the low byte of the chunk's CAT position (HEAD = 0) and P = (17*key % 1925) + 123. So a rewriter can: (1) decrypt the mod once with the real key table (the user's RB20FUL.DAT); (2) add a padding EMBF chunk of at most 1024 bytes (never scrambled), under a name outside the 46 sample names, whose words make both sums land on the chosen fixed offsets (any byte offset in the body); (3) re-scramble every chunk past byte 1024 against the fixed table, keyed by each chunk's new position.

The minimum: steering both sums to the same 2048-byte window needs 2048 bytes. Fewer may be possible by placing every scrambled chunk at a CAT position whose key has a short period P (key 0 gives P = 123, key 114 gives 136, key 227 gives 132; key parity picks the table half), padding with tiny unscrambled chunks; that needs many extra chunks, which the original may reject. Find the smallest amount that the original app still accepts, and choose the window (e.g. one a shipped mod already uses). Embed it in rb (a generated constant with its source offset and the RB20FUL.DAT SHA-256 it was taken from, checked against RB_RB20FUL by a gated test).

Work: `rb recrypt IN.rbm -o OUT.rbm [--rb20ful PATH]`; rb's mod loader recognizes a recrypted mod (its sums select the embedded window) and loads it without RB20FUL.DAT; a recrypted mod must load in the original ReBirth unchanged — verify on the test machine that an app export with the recrypted mod equals the export with the original mod (same song, fitted N; tools/rb_export.py, tools/rbhost/fit.py); and the golden test covers recrypted versions of the corpus mods (the 8 mod songs render identically with the recrypted mods and no RB20FUL.DAT). Document the scheme in docs/rbm.md. Never commit a .rbm or more key bytes than the chosen minimum.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 rb embeds only the chosen minimum of RB20FUL.DAT key data, with its source offset and the DAT's SHA-256, checked by a gated test
- [ ] #2 rb recrypt rewrites a .rbm to use only the embedded data, and rb loads recrypted mods without RB20FUL.DAT
- [ ] #3 A recrypted mod loads in the original app and its export equals the original mod's (test machine), and the corpus mod songs render identically with recrypted mods
- [ ] #4 docs/rbm.md documents the scheme and the minimum chosen, with what the original accepted
<!-- AC:END -->
