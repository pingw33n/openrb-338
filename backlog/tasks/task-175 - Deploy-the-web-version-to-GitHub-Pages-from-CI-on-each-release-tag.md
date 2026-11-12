---
id: TASK-175
title: Deploy the web version to GitHub Pages from CI on each release tag
status: To Do
assignee:
  - '@agent-worker'
created_date: 2026-11-03 09:17
updated_date: '2026-11-12T08:13:00Z'
labels:
  - tools
  - docs
milestone: m-6
dependencies:
  - TASK-172
  - TASK-173
  - TASK-174
references:
  - .github/workflows/release.yml
  - .github/workflows/ci.yml
  - scripts/build-web.sh
  - docs/building.md
  - README.md
priority: medium
type: feature
ordinal: 191000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the m-6 goal (the user, 2026-11-03) is the web version hosted on GitHub, at https://pingw33n.github.io/OpenRB-338/ (the repository is public; it was renamed from openrb-338 to OpenRB-338 on 2026-11-12). task-172 builds the static site with scripts/build-web.sh; task-173 builds index.json of the two collections; this task publishes both.

What to build:
- A workflow `.github/workflows/web.yml` (or a job in release.yml) that runs on a release tag v* and by hand (workflow_dispatch): builds dist/web/ with the tag's version (SOURCE_DATE_EPOCH from scripts/version.sh --epoch, as release.yml does), shallow-clones pingw33n/rb-songs and pingw33n/rb-mods at their main HEAD (the mods are about 950 MB; use a blobless or sparse clone if it is faster), runs `orb web-index` with the two commits, puts index.json next to the page, and deploys with actions/upload-pages-artifact and actions/deploy-pages (actions pinned by SHA like the others; permissions pages: write, id-token: write only on that job).
- A `collections` input on workflow_dispatch is not needed; a redeploy by hand picks up new collection commits.
- The user's one-time step, written in docs/building.md and in the task notes: Settings > Pages > Source: GitHub Actions. Until it is done, the deploy job fails with a clear message.
- README.md links to the web version; the About box's links stay as they are.
- docs/building.md describes the workflow, how to redeploy after a collection changes, and the size of the site.

- The Pages address follows the repository name's case: https://pingw33n.github.io/OpenRB-338/. GitHub repository URLs are case-insensitive, but whether Pages serves the project path case-insensitively is not known: after the first deploy, check both https://pingw33n.github.io/OpenRB-338/ and https://pingw33n.github.io/openrb-338/ (and a deep link such as index.json under each), record the HTTP status of each in the notes, and link only the form that works (the exact case) everywhere.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 web.yml builds and deploys on a v* tag and by hand; a by-hand run on the branch (or a dry run with the deploy step skipped) is linked in the notes
- [ ] #2 The deployed page's index.json pins the rb-songs and rb-mods commits the run cloned, and the About box shows the tag's version
- [ ] #3 Actions are pinned by SHA and the Pages permissions are limited to the deploy job
- [ ] #4 README.md links to https://pingw33n.github.io/OpenRB-338/ (the repository name's case); both the exact-case and the lowercase Pages address were checked after the first deploy, with the results in the notes; docs/building.md covers the workflow, the redeploy and the Pages setting
<!-- AC:END -->
