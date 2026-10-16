# A stateful MCP session leaves a pinned endpoint that is gone

- **Status:** carried
- **Upstream:** to file — prepared in the fork: branch `upstream/mcp-unpin-gone-endpoint`; upstream `main` @ `884c2e6e` keeps the pin for the life of the session (`McpHttpClient` in `mcp/upstream/client.rs`). Drops at the re-pin onto a commit containing it
- **Our commit:** the `fix(mcp): drop a stateful session's pin when its endpoint is gone` commit

## Change

A stateful MCP session through a Service backend stays pinned to the endpoint of its first response only while that endpoint lives: before a request, a pin whose endpoint has left the service is dropped (`EndpointSet::has_override`), and a request whose dial to the pinned endpoint fails (no matching endpoint, a connect error or its timeout) drops the pin. The next request is balanced across the live endpoints and pins the session to the one that answers. A request that reached the pinned endpoint keeps the pin, whatever it answered. The pool's connect timeout surfaces as `ProxyError::UpstreamConnectTimeout`, answered like `UpstreamCallTimeout` (504), so a connect timeout is told apart from a request timeout (`agent_pool::Error::is_connect`, `client/mod.rs`, `proxy/mod.rs`)

## Reason

After the eviction of the upstream pod, every request of an already-initialized client answered 500 `Connect: deadline has elapsed` after about 10 s, until the client initialized a new session: the session kept dialing the IP of the pod that was gone ([giantswarm/agentgateway-upstream#90](https://github.com/giantswarm/agentgateway-upstream/issues/90))
