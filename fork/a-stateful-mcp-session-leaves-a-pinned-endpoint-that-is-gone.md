# A stateful MCP session leaves a pinned endpoint that is gone

- **Status:** carried
- **Upstream:** to file — prepared in the fork: branch `upstream/mcp-unpin-gone-endpoint`; upstream `main` @ `884c2e6e` keeps the pin for the life of the session (`McpHttpClient` in `mcp/upstream/client.rs`). Drops at the re-pin onto a commit containing it
- **Our commit:** the `fix(mcp): drop a stateful session's pin when its endpoint is gone` commit

## Change

A stateful MCP session through a Service backend stays pinned to the endpoint of its first response only while that endpoint lives: before a request, a pin whose endpoint has left the service is dropped (`EndpointSet::has_override`), and a request whose dial to the pinned endpoint fails (no matching endpoint, a connect error or its timeout) drops the pin. The next request is balanced across the live endpoints and pins the session to the one that answers. A request that reached the pinned endpoint keeps the pin, whatever it answered. The pool's connect timeout surfaces as `ProxyError::UpstreamConnectTimeout`, answered like `UpstreamCallTimeout` (504), so a connect timeout is told apart from a request timeout (`agent_pool::Error::is_connect`, `client/mod.rs`, `proxy/mod.rs`)

## Reason

A stateful MCP session through a Service-backed target stayed pinned to the pod IP of its first response: once that pod was replaced, every request of the session failed (`500` "no healthy backends", or a connect error after the dial timeout while the old endpoint was still listed) until the client initialized a new session ([giantswarm/agentgateway-upstream#90](https://github.com/giantswarm/agentgateway-upstream/issues/90)). A `static.host` target is an opaque backend and is never pinned
