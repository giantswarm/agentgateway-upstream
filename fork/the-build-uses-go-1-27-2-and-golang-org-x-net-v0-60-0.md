---
id: the-build-uses-go-1-27-2-and-golang-org-x-net-v0-60-0
fork: agentgateway
theme: version-pins
category: pin
status: carried
upstream:
  outcome: drops-at-repin
  refs: []
  exit: drops when upstream builds with 1.27.2 or later
depends_on: []
commit: ""
verified: 2026-10-19
---
# The build uses Go 1.27.2 and golang.org/x/net v0.60.0

- **Status:** carried
- **Upstream:** not for upstream as such: falls away at the re-pin onto the first upstream commit that builds with Go ≥ 1.27.2 and requires `x/net` ≥ v0.60.0 (upstream's dependency automation moves both). Upstream `main` @ `c0354ea4` (2026-10-12) requires `x/net` v0.60.0 and still declares `go 1.27.0` with no `toolchain` line, so the `x/net` half is upstream and the Go 1.27.2 toolchain is what remains
- **Our commit:** the `build: Go 1.27.2 and golang.org/x/net v0.60.0 for govulncheck` commit

## Change

Build with Go 1.27.2 and `golang.org/x/net` v0.60.0 (`toolchain go1.27.2` in `go.mod`, `GO_VERSION` in `pull_request.yml`, `release.yml` and `model-catalog.yml`, the builder image of `.circleci/Dockerfile.controller`; `golangci-lint` v2.14.0 in `tools/go.mod` and `controller/.custom-gcl.yml`, the first release that reads Go 1.27.2 export data)

## Reason

the line's required `govulncheck` lane found 11 advisories reachable from the controller: the standard library's `crypto/tls`, `net/http`, its `http2`, `html/template` and `net/textproto` (fixed in go1.27.2) and `golang.org/x/net`'s `http2` (fixed in v0.60.0). The `toolchain` directive is what the lane's `setup-go` reads from `go.mod`
