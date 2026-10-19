---
id: the-substrate-egress-counts-refusals-per-actor-and-destination
fork: agentgateway
theme: egress-policies
category: feature
status: carried
commits:
  - 'feat(substrate): the egress counts refusals per actor and destination'
upstream:
  outcome: to-file
  refs: []
  exit: 'the upstream change merged and in the pin'
depends_on: [a-policy-denial-names-the-destination-and-where-it-is-allowed]
verified: 2026-10-19
---
# The Substrate egress counts refusals per actor and destination

## Change

Every destination an actor's egress policy refuses is counted in `agentgateway_atenet_egress_refusals_total`, one
series per actor and destination: `ate_atespace`, `ate_actor_name`, `scheme` (`http`, `https` for a decrypted request
or a TLS connection refused by its SNI, `tcp`), `host` (the authority, the SNI or the tcp address, lowercased; a host that is neither a valid name nor an address reads `other`) and `port` (the
port the actor dialed). The three policy denials record it (`http/substrate/egress.rs` `record_refusal`: `apply` for a
request, `authorize_tls` for a ClientHello, `authorize_tcp` for a connection); a denial for another reason (no
identity, an upgrade, an unavailable control API) is not a refusal of a destination and is not counted. The actor names
the hosts, so the series are bounded (`telemetry/metrics.rs` `EgressRefusals`): an actor's destinations beyond 16 fold
into `host="other"` and `port="0"`, all series beyond 4096 into `ate_actor_name="other"`, and a series no refusal
touched for an hour is dropped from the exporter (the scraper keeps its samples).

## Reason

The gateway decides every refusal and logged none above debug, and its only metric was `requests_total` with status
`403`, which names no actor: the people who use and write an agent could not be shown which host their Session was
refused. The actor in the series is what lets a refusal be shown on the Session or run that met it.

## History

Upstream: to file — prepared in the fork: branch `upstream/egress-refusals-per-actor`.
