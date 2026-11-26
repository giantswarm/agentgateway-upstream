---
id: a-policy-denial-names-the-destination-and-where-it-is-allowed
fork: agentgateway
theme: egress-policies
category: feature
status: carried
commits:
  - 'feat(substrate): a policy denial names the destination and where it is allowed'
  - 'test(substrate): a denial''s status line carries its message'
upstream:
  outcome: open
  refs: [agentgateway/agentgateway#4131]
  exit: "agentgateway/agentgateway#4131 settles how a refusal reason reaches the actor; the change that implements it merged and in the pin"
depends_on: []
verified: 2026-11-26
---
# A policy denial names the destination and where it is allowed

## Change

A destination outside the actor's egress policy is answered `403` with a body that names the scheme, the host
and the port, and where it is allowed (`http/substrate/egress.rs` `destination_denied`, used by `matching_rule`
for an HTTP or a decrypted HTTPS request and by `proxy/httpproxy.rs` for a TLS connection whose SNI no rule
covers, named by that SNI): `actor egress policy denied https to example.org:443: it is not in this agent's
egress policy; the agent's author can allow it in the agent's egress settings`. A refused tcp destination carries
the same remedy. On HTTP/1.1 every `SubstrateEgressDenied` answer also carries its message as the status line's
reason phrase (`proxy/mod.rs`, hyper's `ReasonPhrase`), since many clients an agent runs (Python's `urllib`,
`wget`) print the status line and drop the body. Every other denial keeps its message.

## Reason

The body was `actor egress policy denied TLS destination` (or `… destination`), with no host: an agent's tool
saw a bare `403 Forbidden` and the agent answered with a vague failure ("the server blocks automated access"), so the people who use and write the agent could not
tell which host to declare. The actor named the destination itself, so the body tells it nothing new.

## History

Upstream: to file — prepared in the fork: branch `upstream/egress-denial-names-host-v3` (DCO signed off, with the test change; `upstream/egress-denial-names-host` and `-v2` are superseded and not filed).

2026-10-26: the refusal reason went upstream as the issue agentgateway/agentgateway#4131 on 2026-10-24 (extend the ingress `ate-error-reason` header to the egress, with fixed values); this row's destination echo is its optional second ask. The branch `upstream/egress-denial-names-host-v3` is dead.

2026-11-26, re-pin onto upstream v1.7.0: re-seated on agentgateway#4093 (`42c0902c`, in v1.7.0), which refuses a TLS destination no rule covers at the ClientHello instead of answering 403 after the handshake, and agentgateway#2162 (`be8864a8`), which answers `SubstrateEgressDenied` with the fixed external message `actor egress policy denied`. The denial of an HTTP or a decrypted HTTPS request is now `ProxyError::SubstrateEgressDestinationDenied`, whose external message is the destination text, the same way #4093's `SubstrateEgressUpgradeDenied` exposes its own; only that variant carries the reason phrase. The TLS denial at the ClientHello has no HTTP answer to carry a message any more, and the tcp denial stays an internal log line. Upstream: agentgateway/agentgateway#4131 open, no maintainer answer.
