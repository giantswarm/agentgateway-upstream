---
id: google-golang-org-grpc-is-pinned-at-v1-83-2
fork: agentgateway
theme: version-pins
category: pin
status: dropped
commits:
  - 'build: pin google.golang.org/grpc to v1.83.2 (GO-2026-6443)'
  - 'build: return google.golang.org/grpc to upstream''s v1.84.0'
upstream:
  outcome: fork-only
  refs: [agentgateway/agentgateway#3658, agentgateway/agentgateway#4028, grpc/grpc-go#9440, golang/vulndb#6580]
  exit: 'file deleted at the next re-pin, which replays the pair to no change'
  class: 1
  destination: ""
  first_step: 'none; the next re-pin replays the pair to nothing and deletes this file'
  source: n/a
  test: 'go.mod:48 equals upstream main'
  recheck: 'at the re-pin'
  owner: unassigned
  draft: ""
  decision: issuecomment-6301335159
depends_on: []
verified: 2026-10-20
---
# google.golang.org/grpc is pinned at v1.83.2

## Change

Pin `google.golang.org/grpc` at v1.83.2 in `go.mod` (`build: pin google.golang.org/grpc to v1.83.2 (GO-2026-6443)`), reverted by `build: return google.golang.org/grpc to upstream's v1.84.0`, which puts `go.mod`, `go.sum` and the OSS compliance list back on the pin's v1.84.0.

## Reason

upstream #3614 (in the pin) raised grpc to v1.84.0, which [GO-2026-6443](https://pkg.go.dev/vuln/GO-2026-6443) (a server panic on a request without `:authority` and `Host`, GHSA-2v4p-qf9q-27wj) listed as affected: the vulnerability database named the fix as `v1.85.0-dev` only and v1.83.2 as fixed. grpc's maintainers dispute the entry, stating v1.84.0 already contains the fix (grpc/grpc-go#9440; golang/vulndb#6580). The line's `govulncheck` lane is a required check of `ci-ok`, and the controller's xDS server is the flagged call path.

## History

Upstream ignored the advisory in `osv-scanner.toml` (agentgateway#3658, in the pin); that file steers osv-scanner only and `govulncheck` reads no exclusion list, so the line pinned v1.83.2 in the 2026-09-24 re-pin's own commit `ba219016`, replayed clean at the 2026-10-05 re-pin (v1.6.0 still requires v1.84.0).

GO-2026-6443 was revised on 2026-10-06: its ranges now close the 1.84 development line at `1.84.0-dev.0.20260825144003-d5a41119e0e3`, so the released v1.84.0 is outside them, and upstream `main` dropped its `osv-scanner.toml` exclusion in agentgateway#4028. golang/vulndb#6580 is still open. On 2026-10-20 the pin was reverted on the line: `govulncheck ./...` on v1.84.0 reports no GO-2026-6443, and the controller builds. Both commits stay on `giantswarm` until the next re-pin, which replays the pair to no change and leaves this file to be deleted.

2026-10-20: decided in the version-pins theme review (decision issuecomment-6301335159): class 1 drop; the pair nets to upstream's v1.84.0 and this file is deleted at the next re-pin.
