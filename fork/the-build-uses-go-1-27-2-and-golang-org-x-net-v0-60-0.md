---
id: the-build-uses-go-1-27-2-and-golang-org-x-net-v0-60-0
fork: agentgateway
theme: version-pins
category: pin
status: carried
commits:
  - 'build: Go 1.27.2 and golang.org/x/net v0.60.0 for govulncheck'
  - 'build: build the custom golangci-lint at v2.14.0 for Go 1.27.2'
  - 'build: Go 1.27.3 for govulncheck (GO-2026-6809)'
upstream:
  outcome: drops-at-repin
  refs: [agentgateway/agentgateway#3865, agentgateway/agentgateway#4063]
  exit: 'drops at the re-pin onto a commit containing 7343b450 (#4063: toolchain go1.27.3, lanes on go-version-file); the builder image in .circleci/Dockerfile.controller is the line''s own'
  class: 2
  destination: 'agentgateway/agentgateway#3865 + #4063 (7343b450, on upstream main)'
  first_step: 'none; the builder image in .circleci/Dockerfile.controller is on 1.27.3 since 2026-10-22'
  source: n/a
  test: 'the govulncheck job of pull_request.yml is green on the re-pin'
  recheck: 'at the re-pin'
  owner: unassigned
  draft: ""
  decision: issuecomment-6301335159
depends_on: []
verified: 2026-10-22
---
# The build uses Go 1.27.3 and golang.org/x/net v0.60.0

## Change

Build with Go 1.27.3 and `golang.org/x/net` v0.60.0 (`toolchain go1.27.3` in `go.mod`, `GO_VERSION` in `pull_request.yml`, `release.yml` and `model-catalog.yml`, the builder image of `.circleci/Dockerfile.controller`; `golangci-lint` v2.14.0 in `tools/go.mod` and `controller/.custom-gcl.yml`, the first release that reads Go 1.27.2 export data)

## Reason

the line's required `govulncheck` lane found 11 advisories reachable from the controller: the standard library's `crypto/tls`, `net/http`, its `http2`, `html/template` and `net/textproto` (fixed in go1.27.2) and `golang.org/x/net`'s `http2` (fixed in v0.60.0); then GO-2026-6809 in `crypto/x509` (fixed in go1.27.3). The `toolchain` directive is what the lane's `setup-go` reads from `go.mod`

## History

not for upstream as such: falls away at the re-pin onto the first upstream commit that builds with Go ≥ 1.27.2 and requires `x/net` ≥ v0.60.0 (upstream's dependency automation moves both). Upstream `main` @ `c0354ea4` (2026-10-12) requires `x/net` v0.60.0 and still declares `go 1.27.0` with no `toolchain` line, so the `x/net` half is upstream and the Go 1.27.2 toolchain is what remains

the `build: Go 1.27.2 and golang.org/x/net v0.60.0 for govulncheck` commit

2026-10-20: decided in the version-pins theme review (decision issuecomment-6301335159): class 2 upstream-has-it: upstream builds CI on 1.27.2 since #3865 (2026-10-12, with x/net v0.60.0 and golangci-lint v2.14.0) and pins `toolchain go1.27.3` since #4063 (merged 2026-10-20, `7343b450`), so the go.mod toolchain line drops at the re-pin onto a commit containing it; the builder image pin is the line's own and moves to 1.27.3 as fork infra.

2026-10-22: moved to Go 1.27.3 for GO-2026-6809 (`crypto/x509`, fixed in go1.27.3), which turned the line's required `govulncheck` lane red on every pull request; upstream pins `toolchain go1.27.3` since agentgateway#4063 (`7343b450`, merged 2026-10-20), the `build: Go 1.27.3 for govulncheck (GO-2026-6809)` commit
