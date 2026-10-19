---
id: google-golang-org-grpc-is-pinned-at-v1-83-2
fork: agentgateway
theme: version-pins
category: pin
status: carried
upstream:
  outcome: fork-only
  refs: [agentgateway/agentgateway#3658, golang/vulndb#6580]
  exit: drop the downgrade
depends_on: []
commit: ba219016
verified: 2026-10-19
---
# google.golang.org/grpc is pinned at v1.83.2

- **Status:** carried
- **Upstream:** not for upstream as such: falls away at the re-pin onto the first upstream commit that requires a grpc release with the fix (upstream bumps grpc through its dependency automation; a grpc release of the 1.84 line or v1.85.0 closes it). Upstream ignores the advisory in `osv-scanner.toml` (agentgateway#3658, in the pin); that file steers osv-scanner only and `govulncheck` reads no exclusion list, so the exclusion cannot replace the pin while golang/vulndb#6580 stays open
- **Our commit:** `ba219016` (the 2026-09-24 re-pin's own commit, replayed clean at the 2026-10-05 re-pin: v1.6.0 still requires v1.84.0, and the controller builds and its translator tests pass on v1.83.2)

## Change

Pin `google.golang.org/grpc` at v1.83.2 in `go.mod` (`build: pin google.golang.org/grpc to v1.83.2 (GO-2026-6443)`)

## Reason

upstream #3614 (in the pin) raised grpc to v1.84.0, which carries [GO-2026-6443](https://pkg.go.dev/vuln/GO-2026-6443) (a server panic on a request without `:authority` and `Host`, GHSA-2v4p-qf9q-27wj): the vulnerability database names the fix as `v1.85.0-dev` only and v1.83.2 as fixed; grpc's maintainers dispute the entry, stating v1.84.0 already contains the fix (grpc/grpc-go#9440; golang/vulndb#6580 open). The line's `govulncheck` lane is a required check of `ci-ok`, and the controller's xDS server is the flagged call path
