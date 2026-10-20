---
id: the-conformance-suite-runs-gateway-api-conformance-v1-6-3
fork: agentgateway
theme: test-flakes
category: test
status: carried
commits:
  - 'build(deps): avoid depending on GIE conformance in go.mod'
upstream:
  outcome: merged
  refs: [agentgateway/agentgateway#3978, kubernetes-sigs/gateway-api#5110]
  exit: next re-pin
depends_on: []
verified: 2026-10-20
---
# The conformance suite runs gateway-api conformance v1.6.3

## Change

`go.mod` requires `sigs.k8s.io/gateway-api` and `sigs.k8s.io/gateway-api/conformance` at v1.6.3 (v1.6.2 and v1.6.1 on the pin) and no longer requires `sigs.k8s.io/gateway-api-inference-extension/conformance`; `controller/test/conformance/conformance_test.go` drops its blank import, and `controller/Makefile` and `controller/hack/bump_deps.sh` run the GIE conformance suite without it in `go.mod`.

## Reason

`TLSRouteInvalidBackendRefNonexistent` failed `controller-conformance` three times between 2026-10-19 and 2026-10-20, once on a push to `giantswarm`. Conformance v1.6.1's `MakeTLSConnectionAndExpectEventuallyConnectionRejection` dials with the test's context and no deadline of its own, so a first dial to a Gateway address that is not serving yet hangs for the whole `MaxTimeToConsistency` (30s) and is never retried. kubernetes-sigs/gateway-api#5110 bounds each attempt by `RequestTimeout`; it is in v1.6.2 and later.

## History

`git cherry-pick -x` of upstream `a6698884` (agentgateway#3978, merged 2026-10-14, after the v1.6.0 pin). Falls away at the re-pin onto a commit that contains it.
