# The gRPC status of a trailers-only refusal is logged

- **Status:** carried
- **Upstream:** filed as [agentgateway#3873](https://github.com/agentgateway/agentgateway/pull/3873) (open, 2026-10-13; `proxy/httpproxy.rs` and `telemetry/log.rs`). Not in #3873 and not filed: the same gap on the request limit refusal (`RequestLimitExceeded` in `proxy/gateway.rs`, answered outside `resolve_response`). Upstream `main` @ `c0354ea4` still sets `grpc.status` from an upstream's headers and trailers only. Drops at the re-pin onto a commit containing #3873
- **Our commit:** the `fix(proxy): log the gRPC status of a refusal answered trailers-only` commit

## Change

Log the gRPC status of a refusal the proxy answers trailers-only (`resolve_response` in `proxy/httpproxy.rs`)

## Reason

a JWT refusal of a gRPC request is HTTP 200 with `grpc-status: 16` in the headers, and the access log showed `http.status=200` only, so an authentication failure read as a success ([#46](https://github.com/giantswarm/agentgateway-upstream/issues/46))
