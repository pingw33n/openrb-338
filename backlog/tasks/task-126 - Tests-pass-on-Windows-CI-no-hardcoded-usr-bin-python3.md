---
id: TASK-126
title: 'Tests pass on Windows CI: no hardcoded /usr/bin/python3'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 15:43'
labels:
  - tests
  - tools
milestone: m-4
dependencies: []
references:
  - .github/workflows/ci.yml
  - crates/rb-engine/tests/rb1x_messages.rs
  - docs/building.md
priority: high
type: bug
ordinal: 145000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the first CI run (2026-10-28, https://github.com/pingw33n/rb/actions/runs/40715817032) failed on windows-latest. macOS, ubuntu and fmt passed.
- The first failure is crates/rb-engine/tests/rb1x_messages.rs:107: 'cannot run /usr/bin/python3: ... NotFound'.
- Cargo stops at the first failing test binary, so further Windows failures may be hidden.

The hardcoded interpreter /usr/bin/python3 is used in:
- crates/rb-cli/tests/thirdparty.rs:80 and :423;
- crates/rb-engine/tests/rb1x_messages.rs:93;
- crates/rb-engine/tests/rbsmsg_corpus.rs:78;
- crates/rbm/tests/rbm.rs:354;
- crates/rbs/tests/corpus.rs:103.

What:
1. One shared way to find Python, used by all of these:
   - RB_PYTHON if set;
   - else the first of python3, python, py -3 on PATH that runs and is Python 3.
   Keep -I (isolated mode). A test that needs Python and finds none fails with a clear message naming RB_PYTHON. Don't silently skip: CI must run them, and every runner has Python 3.
2. Audit the tests for other Unix-only assumptions:
   - path separators or '/'-joined paths;
   - chmod / permission tests: they must be cfg(unix) or have a Windows equivalent;
   - line endings in expected text (CRLF from git checkout: add .gitattributes if committed fixtures are compared byte for byte);
   - temp dir naming, symlinks, and executable names (rb vs rb.exe, via env!("CARGO_BIN_EXE_rb")).
3. Make the CI test step run with --no-fail-fast so one run shows every failure.
4. Verify on Windows. Either:
   - push nothing yourself; the supervisor pushes and watches CI; or
   - build and run the test binaries on the Windows test machine under the brief's permits (C:\re\rb\<folder>, delete it after).
   At least cross-compile the tests for x86_64-pc-windows-msvc with cargo-xwin (cargo xwin test --no-run) to catch compile errors.
5. Document RB_PYTHON in docs/building.md and AGENTS.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No test hardcodes an interpreter path; RB_PYTHON or python3/python/py on PATH is used through one helper
- [ ] #2 CI's test step uses --no-fail-fast; the Unix-only tests are cfg-gated or portable; fixtures compared byte for byte are protected from CRLF conversion
- [ ] #3 The workspace tests pass on Windows (the test machine, or the supervisor's CI run recorded in the notes); cargo +nightly fmt --check, clippy and cargo test pass on macOS
<!-- AC:END -->
