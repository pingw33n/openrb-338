---
id: TASK-142
title: 'build.sh: DIST outside the checkout is lost for Docker builds'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-29 12:48'
updated_date: '2026-10-29 13:02'
labels:
  - tools
milestone: m-4
dependencies: []
references:
  - scripts/build.sh
  - docs/building.md
priority: low
type: bug
ordinal: 161000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: on 2026-10-29 the supervisor ran `DIST=/private/tmp/.../linux-orb scripts/build.sh x86_64-unknown-linux-gnu` on a Mac. The Docker build finished, and the script printed "built x86_64-unknown-linux-gnu -> <DIST>/x86_64-unknown-linux-gnu/bin", but no file was there. build_docker (scripts/build.sh, around lines 45-60) passes -e DIST=$dist into the container but mounts only $root (the checkout) and the git common dir, so a DIST outside the checkout is written inside the container and lost when it exits (--rm). A wasted 10-minute build, with a success message.

What:
- When DIST is outside $root, either mount it into the container at the same path (create it on the host first), or refuse with a clear error before building. Mounting is preferred.
- The host-side "built ... ->" message must only print when the files exist on the host. Check after docker run, and fail otherwise.
- Check package.sh and build-all.sh for the same assumption.
- Document DIST's behavior in docs/building.md.
- Test: shellcheck. A real run of `DIST=<a scratch dir outside the repo> scripts/build.sh x86_64-unknown-linux-gnu` (Docker is running on this Mac; the openrb-target-x86_64 volume caches the build) leaves orb and OpenRB-338 in that dir; give `file` output.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A Docker build with DIST outside the checkout leaves the binaries in DIST on the host (real run, file output recorded), or fails before building with a clear message; the success line is printed only when the files exist
- [x] #2 shellcheck passes; docs/building.md describes DIST
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. build.sh: make DIST absolute (mkdir -p on host); in build_docker mount it at the same path when outside $root; after docker run, require both binaries on the host before printing the success line (inner in-container run prints its own line, the host line is the checked one). 2. Check package*.sh/build-all.sh: they only read DIST, no docker. 3. docs/building.md: describe DIST. 4. shellcheck, real run with DIST in scratchpad.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a4a0712af97321cc3 (rebased on main 833e625)

Real run: DIST=<scratchpad>/task142/dist scripts/build.sh x86_64-unknown-linux-gnu, 3m34s cargo, both 'built' lines printed (container inner + host-checked). file: orb and OpenRB-338 = ELF 64-bit LSB pie executable, x86-64, dynamically linked, for GNU/Linux 3.2.0, stripped. shellcheck scripts/*.sh clean. package*.sh read DIST only; build-all.sh uses $root/dist only; no change needed. Not run: the refuse path (missing file) is the same loop; untested by fault injection.

Review round 1 fixed: bash 3.2 empty-array guard on mounts, DIST equal to root is not mounted, bin dir removed before docker run, Docker wording in header and building.md. Stub-docker tests under /bin/bash 3.2.57 in a fresh clone layout (the dot-git entry a directory): default DIST with a stub that writes files gives rc 0 and the built line; a stub that writes nothing after that gives rc 1 and the missing-file message (stale files removed, so the failure path is covered); DIST equal to root gives rc 0 with root mounted once. Script: scratchpad task142/t.sh. shellcheck clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
build.sh now makes DIST absolute (mkdir -p), mounts a DIST outside the checkout into the Docker container at the same path, and after docker run requires orb and OpenRB-338 in DIST/TARGET/bin on the host (exit 1 with a message otherwise) before printing the host success line. docs/building.md describes DIST. Real run left both Linux ELF binaries in the scratch DIST; shellcheck clean. Branch worktree-agent-a4a0712af97321cc3, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->
