---
id: TASK-141
title: 'release.yml: a bad tag passes prepare (tee hides the release-env.sh failure)'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-29 12:43'
updated_date: '2026-10-29 12:45'
labels:
  - tools
milestone: m-4
dependencies: []
references:
  - .github/workflows/release.yml
  - scripts/release-env.sh
  - scripts/version.sh
  - scripts/release-assets.sh
priority: medium
type: bug
ordinal: 160000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the first release checks (task-134, 2026-10-29) pushed the bad tag v1 on purpose. release.yml must fail in the prepare job, so that nothing is built and no release is created. Instead, run 40873523255 passed prepare, built all 12 packages (about 25 runner-minutes) and failed only in the draft release job, at `scripts/release-assets.sh release-in "$TAG"` ("not a release tag: v1"). No release was created, so the result was safe, but the early stop the comment in release.yml promises did not happen.

Cause (to be confirmed): the prepare step runs `scripts/release-env.sh ${tag:+"$tag"} | tee -a "$GITHUB_OUTPUT"` with GitHub's default shell for a step without `shell:`, which is `bash -e {0}` (the run log shows "shell: /usr/bin/bash -e {0}"). That shell has no pipefail, so the pipeline's status is tee's, and release-env.sh exiting 1 on the bad tag (version.sh --epoch rejects it) is ignored. The packages were then built with an empty SOURCE_DATE_EPOCH output.

What:
- Make prepare fail on a bad tag: set `shell: bash` (which GitHub runs as bash --noprofile --norc -eo pipefail), or drop the pipe and write the output file another way. Check every other `run:` step in release.yml and ci.yml for the same pipe-hides-failure pattern, and fix those too, or set `defaults: run: shell: bash` for the workflow.
- Also make the later jobs refuse an empty source_date_epoch, as a second guard.
- Test locally: run release-env.sh with v1 and with v2026.1029.1138 and check the exit codes. actionlint must pass. Write the command that reproduces the old behavior in the notes, for example `bash -e -c 'scripts/release-env.sh v1 | tee /dev/null'; echo $?`.
- The real check is one more bad-tag run on GitHub. The supervisor runs it after the merge, with the user's go-ahead, and records it in task-134 AC #3.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 prepare fails on a tag that is not v<YYYY.MMDD.HHMM> (the release-env.sh failure is no longer hidden by a pipe); every run step that pipes uses pipefail; later jobs refuse an empty source_date_epoch
- [x] #2 release-env.sh exit codes checked locally for v1 and a good tag; actionlint and shellcheck pass; the GitHub bad-tag re-run is noted in task-134 for the supervisor
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. release.yml: workflow-level defaults run shell bash (bash -eo pipefail) so every step with a pipe (prepare tee, changelog tee) fails with its first command. 2. Guard step 'Refuse an empty build time' first in package, dmg, installer, appimage (the jobs that take source_date_epoch). 3. ci.yml has no pipes: unchanged. 4. Check locally: release-env.sh v1 / good tag, actionlint, shellcheck.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-ad683df01ca3dd2f5

Cause confirmed: run 40873523255 prepare log shows 'shell: /usr/bin/bash -e {0}' (no pipefail). Old behavior reproduced: bash -e -c 'scripts/release-env.sh v1 | tee /dev/null'; echo $? -> prints the version.sh error, exit 0. With pipefail (bash -eo pipefail -c, what 'shell: bash' runs) exit 1. Good tag v2026.1029.1138: exit 0, SOURCE_DATE_EPOCH=1793273880. actionlint exit 0, shellcheck scripts/*.sh exit 0. ci.yml: only plain run lines, no pipes; unchanged. release.yml pipes: prepare tee and the changelog tee, both now under defaults.run.shell bash. The release job does not use the epoch; it needs package, so the guard in package/dmg/installer/appimage covers it. The GitHub bad-tag re-run is the supervisor's (task-134 AC 3).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
release.yml: workflow-level defaults.run.shell bash (pipefail) fixes the prepare tee that hid the release-env.sh failure on a bad tag, and the changelog tee; a first step in package, dmg, installer and appimage fails on an empty SOURCE_DATE_EPOCH. Local: release-env.sh v1 exits 1 through a pipefail pipe (0 under bash -e, the old behavior), good tag exits 0; actionlint and shellcheck pass. AC 2's GitHub bad-tag re-run is left to the supervisor, noted in task-134. Branch worktree-agent-ad683df01ca3dd2f5.
<!-- SECTION:FINAL_SUMMARY:END -->
