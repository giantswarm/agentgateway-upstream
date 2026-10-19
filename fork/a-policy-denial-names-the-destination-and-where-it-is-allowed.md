---
id: a-policy-denial-names-the-destination-and-where-it-is-allowed
fork: agentgateway
theme: egress-policies
category: feature
status: carried
commits:
  - 'feat(substrate): a policy denial names the destination and where it is allowed'
upstream:
  outcome: to-file
  refs: []
  exit: 'the upstream change merged and in the pin'
depends_on: []
verified: 2026-10-19
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

Upstream: to file — prepared in the fork: branch `upstream/egress-denial-names-host`.
