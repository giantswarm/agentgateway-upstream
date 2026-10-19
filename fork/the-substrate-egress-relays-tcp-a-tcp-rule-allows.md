# The Substrate egress relays TCP a tcp rule allows

- **Status:** carried
- **Upstream:** to file — prepared in the fork: branch `upstream/substrate-egress-tcp-rule`. It follows the Substrate API's `tcp` rule, prepared on the Substrate line's branch `upstream/egress-tcp-rule` ([giantswarm/substrate#164](https://github.com/giantswarm/substrate/issues/164)).
- **Our commit:** the `feat(substrate): relay TCP a tcp rule allows` commit on `giantswarm`

## Change

`EgressRule` in `crates/protos/proto/ateapi.proto` gains `tcp` (field 4, `TCPRule`: `hostnames`, `cidrs`, `ports`). An actor stream on the internal `AUTO` bind that is neither a TLS ClientHello nor an HTTP request reaches `proxy_tcp` (`crates/agentgateway/src/proxy/gateway.rs`), which no longer closes it outright: `substrate::authorize_tcp` (`crates/agentgateway/src/http/substrate/egress.rs`) fetches the actor's policy and allows it when a `tcp` rule on the CONNECT's port names a CIDR containing the CONNECT's address, or a hostname the gateway's resolver (`Client::resolve_all`, every address of the cached answer) resolves to it. The backend is pinned to that address (`DynamicBackendOverride`), whatever the TCP route names. Only a CONNECT that named an IP literal carries the address (`EgressConnectAddress`); unspecified, multicast and broadcast addresses are refused, and loopback and link-local ones (the gateway's and its node's, a cloud metadata service among them) are reached only through a CIDR inside those ranges, never through a hostname or a wider CIDR. A refused stream is closed with no bytes, as before. A rule with more than one union member allows nothing, and the other members' matching skips a rule that sets `tcp`. Unit tests in `egress.rs` and `client/dns_tests.rs`; `substrate_egress_relays_raw_tcp_a_tcp_rule_allows` in `tests/tests/substrate.rs` relays a client-first exchange on a listed port and closes an unlisted one without dialing.

## Reason

The Substrate line carries the `tcp` rule ([giantswarm/substrate#164](https://github.com/giantswarm/substrate/issues/164)): an agent uses git over SSH or a database whose protocol is not TLS only when its author declares the host and port. agentgateway is that line's egress dataplane, so the rule takes effect only with this change; without it every such stream is closed.

Drops at the re-pin onto an upstream commit that contains the upstream change.
