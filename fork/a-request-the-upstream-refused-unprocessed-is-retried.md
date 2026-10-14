# A request the upstream refused unprocessed is retried on a fresh connection

- **Status:** carried
- **Upstream:** prepared for upstream as branch `upstream/retry-unprocessed-h2-stream`, not opened; upstream `main` has no retry of a refused HTTP/2 stream (searched 2026-10-14: "refused", "goaway", "unprocessed")
- **Our commit:** the `fix(proxy): retry a request the upstream refused before processing it` commit

## Change

A request whose HTTP/2 stream the upstream reset with `REFUSED_STREAM`, or which sat above the last stream id of the upstream's `GOAWAY`, is retried once on a fresh connection when no retry policy applies, whatever its method; a retry policy counts it as a retryable upstream failure as before. The pool retires the connection that refused the stream, so the retry opens a new one; the streams already on it run to completion (`agent_pool::Error::is_unprocessed`, `H2CapacityGuard::retire` in `crates/pool`, the attempt loop in `proxy/httpproxy.rs`)

## Reason

Substrate's egress gateway answered `503` to an actor's `POST .../git-upload-pack` to github.com when a pooled HTTP/2 connection the upstream was closing refused the new stream, and the actor's skill fetch failed ([giantswarm/substrate#401](https://github.com/giantswarm/substrate/issues/401)). A refused stream was never processed by the server, so RFC 9113, section 8.7, makes it safe to retry
