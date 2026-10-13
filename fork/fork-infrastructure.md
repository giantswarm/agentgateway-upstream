# Fork infrastructure

- **Status:** carried
- **Upstream:** not for upstream; ours to keep
- **Our commit:** the `giantswarm` branch history

## Change

Fork infrastructure: this file, the README pointer, `CODEOWNERS`, `.circleci/config.yml` and `.circleci/Dockerfile.controller` (the publishing pipeline, see "Publishing"), `.github/workflows/sync-upstream.yaml`, `.trivyignore`; `pull_request.yml` on `giantswarm` and on `sync/**` pushes (a re-pin candidate is a rebased branch, so a pull request from it runs no `pull_request` workflow — the push runs `ci-ok`) with GitHub-hosted runners, Linux lanes only, `govulncheck` and `ci-ok`; `release.yml` without its tag and `workflow_dispatch` triggers and `nightly.yml` without its `workflow_dispatch` trigger (upstream's registry pushes stay in the text, unreachable: no event runs them here — see "Publishing")

## Reason

the line's CI, publishing and sync
