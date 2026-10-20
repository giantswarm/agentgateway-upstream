---
id: the-grpc-status-of-a-trailers-only-refusal-is-logged
fork: agentgateway
theme: credential-injection
category: bugfix
status: carried
commits:
  - 'fix(proxy): log the gRPC status of a refusal answered trailers-only'
upstream:
  outcome: merged
  refs: [agentgateway/agentgateway#3873]
  exit: "drops at the re-pin onto a commit containing 1370236e; the request-limit refusal that writes no access-log line is an optional issue"
depends_on: []
verified: 2026-10-20
---
# The gRPC status of a trailers-only refusal is logged

## Change

Log the gRPC status of a refusal the proxy answers trailers-only (`resolve_response` in `proxy/httpproxy.rs`)

## Reason

a JWT refusal of a gRPC request is HTTP 200 with `grpc-status: 16` in the headers, and the access log showed `http.status=200` only, so an authentication failure read as a success ([#46](https://github.com/giantswarm/agentgateway-upstream/issues/46))

## History

merged as [agentgateway#3873](https://github.com/agentgateway/agentgateway/pull/3873) (`1370236e`, merged 2026-10-13 by howardjohn; `proxy/httpproxy.rs` and `telemetry/log.rs`). Not in #3873 and not filed: the same gap on the request limit refusal (`RequestLimitExceeded` in `proxy/gateway.rs`, answered outside `resolve_response`). Drops at the re-pin onto a commit containing `1370236e`

the `fix(proxy): log the gRPC status of a refusal answered trailers-only` commit

2026-10-20, decided on internal issue 38652, comment 6301541135: class 2 upstream has it; destination agentgateway/agentgateway#3873; first step none (an optional issue on the request-limit refusal, which writes no access-log line upstream); branch none; owner unassigned; draft: internal issue 38652, comment 6301548312.
