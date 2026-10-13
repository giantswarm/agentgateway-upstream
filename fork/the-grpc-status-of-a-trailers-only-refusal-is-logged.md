# The gRPC status of a trailers-only refusal is logged

- **Status:** carried
- **Upstream:** to file, together with the same gap on the request limit refusal (`RequestLimitExceeded` in `proxy/gateway.rs`, answered outside `resolve_response`): upstream `main` @ `85d1f087` sets `grpc.status` from an upstream's headers and trailers only, never for a response the proxy builds itself (#37742 row 143)
- **Our commit:** the `fix(proxy): log the gRPC status of a refusal answered trailers-only` commit

## Change

Log the gRPC status of a refusal the proxy answers trailers-only (`resolve_response` in `proxy/httpproxy.rs`)

## Reason

a JWT refusal of a gRPC request is HTTP 200 with `grpc-status: 16` in the headers, and the access log showed `http.status=200` only, so an authentication failure read as a success ([#46](https://github.com/giantswarm/agentgateway-upstream/issues/46))
