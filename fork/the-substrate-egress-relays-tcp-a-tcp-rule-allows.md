---
id: the-substrate-egress-relays-tcp-a-tcp-rule-allows
fork: agentgateway
theme: egress-policies
category: feature
status: carried
commits:
  - 'feat(substrate): relay TCP a tcp rule allows'
  - 'fix(substrate): a tcp rule reaches metadata services and the gateway only by a narrow CIDR'
  - 'fix(substrate): the local-use NAT64 prefix is local for a tcp rule'
  - 'fix(substrate): every address of the gateway is local for a tcp rule'
  - 'style(substrate): rustfmt the tcp interface-address test'
upstream:
  outcome: to-file
  refs: [giantswarm/substrate#164]
  exit: file as a PR after the Substrate API's tcp rule is filed upstream
depends_on: [giantswarm/substrate#164]
verified: 2026-10-19
---
# The Substrate egress relays TCP a tcp rule allows

## Change

`EgressRule` in `crates/protos/proto/ateapi.proto` gains `tcp` (field 4, `TCPRule`: `hostnames`, `cidrs`, `ports`). An actor stream on the internal `AUTO` bind that is neither a TLS ClientHello nor an HTTP request reaches `proxy_tcp` (`crates/agentgateway/src/proxy/gateway.rs`), which no longer closes it outright: `substrate::authorize_tcp` (`crates/agentgateway/src/http/substrate/egress.rs`) fetches the actor's policy and allows it when a `tcp` rule on the CONNECT's port names a CIDR containing the CONNECT's address, or a hostname the gateway's resolver (`Client::resolve_all`, every address of the cached answer) resolves to it. The backend is pinned to that address (`DynamicBackendOverride`), whatever the TCP route names. Only a CONNECT that named an IP literal carries the address (`EgressConnectAddress`); unspecified, multicast and broadcast addresses are refused, and the gateway's and its node's addresses (loopback, link-local with the metadata service at 169.254.169.254, AWS's IPv6 metadata service, Alibaba Cloud's metadata service, Azure's WireServer, every address of the gateway's network interfaces and the one the CONNECT arrived on, both families on a dual-stack pod, and their NAT64 forms) are reached only through a CIDR inside those ranges, never through a hostname or a wider CIDR. A refused stream is closed with no bytes, as before. A rule with more than one union member allows nothing, and the other members' matching skips a rule that sets `tcp`. Unit tests in `egress.rs` and `client/dns_tests.rs`; `substrate_egress_relays_raw_tcp_a_tcp_rule_allows` in `tests/tests/substrate.rs` relays a client-first exchange on a listed port and closes an unlisted one without dialing.

## Reason

The Substrate line carries the `tcp` rule ([giantswarm/substrate#164](https://github.com/giantswarm/substrate/issues/164)): an agent uses git over SSH or a database whose protocol is not TLS only when its author declares the host and port. agentgateway is that line's egress dataplane, so the rule takes effect only with this change; without it every such stream is closed.

Drops at the re-pin onto an upstream commit that contains the upstream change.

## History

to file — prepared in the fork: branch `upstream/substrate-egress-tcp-rule-v3` (every address of the gateway local, both families). Branches `upstream/substrate-egress-tcp-rule` and `upstream/substrate-egress-tcp-rule-v2` are superseded by it and are not filed. It follows the Substrate API's `tcp` rule, prepared on the Substrate line's branch `upstream/egress-tcp-rule` ([giantswarm/substrate#164](https://github.com/giantswarm/substrate/issues/164)).

the `feat(substrate): relay TCP a tcp rule allows` commit on `giantswarm`
