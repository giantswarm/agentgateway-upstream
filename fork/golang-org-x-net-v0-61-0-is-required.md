---
id: golang-org-x-net-v0-61-0-is-required
fork: agentgateway
theme: version-pins
category: pin
status: carried
commits:
  - 'fix(deps): bump golang.org/x/net to v0.61.0 for two HTTP/2 CVEs'
upstream:
  outcome: drops-at-repin
  refs: []
  exit: 'not replayed at the next re-pin; this file is deleted then'
  class: 1
  destination: ""
  first_step: 'not replayed at the next re-pin'
  source: n/a
  test: 'govulncheck job and trivy image scan clean at upstream''s x/net v0.60.0'
  recheck: 'at the re-pin'
  owner: unassigned
  draft: ""
  decision: issuecomment-6301335159
depends_on: []
verified: 2026-10-20
---
# golang.org/x/net v0.61.0 is required

## Change

Require `golang.org/x/net` v0.61.0 in `go.mod` and `tools/go.mod` (`go get golang.org/x/net@v0.61.0 && go mod tidy` in each; `x/sync`, `x/term`, `x/text` and `x/tools` move along, and the controller's generated license table `hack/utils/oss_compliance/osa_provided.md` names the new `x/sync` and `x/tools`, which `make verify` checks)

## Reason

CVE-2026-97032 (GO-2026-6617) and CVE-2026-78663 (GO-2026-6612) were read as fixed in `x/net` v0.61.0; both are fixed in v0.60.0, which upstream `main` requires, and no advisory affects v0.60.0. The line has no nancy step: its gates are the trivy image scan and the `govulncheck` job of `pull_request.yml`, both clean at v0.60.0

## History

not for upstream as such: falls away with the row above at the re-pin onto the first upstream commit that requires `x/net` ≥ v0.61.0 (upstream's dependency automation moves it)

the `fix(deps): bump golang.org/x/net to v0.61.0 for two HTTP/2 CVEs` commit

2026-10-20: decided in the version-pins theme review (decision issuecomment-6301335159): class 1 drop, no consumer (the CVEs are fixed in v0.60.0, which upstream requires; no nancy step exists on the line); the commit is not replayed at the next re-pin and this file is deleted then.
