---
id: TASK-142
title: 'build.sh: DIST outside the checkout is lost for Docker builds'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-29 12:48'
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
- [ ] #1 A Docker build with DIST outside the checkout leaves the binaries in DIST on the host (real run, file output recorded), or fails before building with a clear message; the success line is printed only when the files exist
- [ ] #2 shellcheck passes; docs/building.md describes DIST
<!-- AC:END -->
