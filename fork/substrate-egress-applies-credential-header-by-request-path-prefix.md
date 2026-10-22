---
id: substrate-egress-applies-credential-header-by-request-path-prefix
fork: agentgateway
theme: credential-injection
category: feature
status: carried
commits:
  - 'fix(substrate): apply a credential header by its request path prefix'
upstream:
  outcome: fork-only
  refs: [agent-substrate/substrate#2515, agent-substrate/substrate#2541, agentgateway/agentgateway#3985]
  exit: "with giantswarm/substrate:egress-scopes-credential-header-to-request-path-prefix (2026-11-17): re-seated as the agentgateway half of the shape Substrate takes, or dropped at the following re-pin"
depends_on: [agent-substrate/substrate#2541, giantswarm/substrate:egress-scopes-credential-header-to-request-path-prefix]
verified: 2026-10-20
---
# The Substrate egress applies a credential header by its request path prefix

## Change

`CredentialHeader` in `crates/protos/proto/ateapi.proto` gains `path_prefix` (field 5). `SubstrateEgress::apply_effects` (`crates/agentgateway/src/http/substrate/egress.rs`) applies, per header name (case-insensitive), the `replace_headers` entry with the longest `path_prefix` the request path lies under, the entry without a prefix otherwise, none if neither, and fetches only the chosen entry's credential (`replace_headers_for`). A prefix matches the path without its query at a segment boundary; a path with a dot or empty segment (a percent-encoded `/` read as a separator too, `substrate-egress-path-prefix-admits-an-encoded-slash.md`), `..;`, a backslash, or a percent-encoded `.` or `\` matches no prefix. The same rules as Substrate's `egresspolicy.ReplaceHeadersFor`, which its Envoy egress applies. A policy without prefixes applies as before. Unit tests mirror Substrate's `TestReplaceHeadersFor` and `TestPathPrefixMatches`.

## Reason

The Substrate line carries `path_prefix` (giantswarm/substrate#349, for giantswarm/substrate#344): one host takes `Basic` for Git over HTTPS and `Bearer` for its REST API under `/api/`. Without this change the egress applied every entry for a header, the last one winning on every path, so a path-scoped credential reached every path of the host and the Substrate line's agentgateway e2e failed `TestActorEgressCredentialInjection`.

Drops at the re-pin onto an upstream commit that contains the upstream change.

## History

to file — prepared in the fork: branch [`upstream/substrate-egress-path-prefix`](https://github.com/giantswarm/agentgateway-upstream/tree/upstream/substrate-egress-path-prefix) on upstream `main` @ `50067185`. It follows [agent-substrate/substrate#2515](https://github.com/agent-substrate/substrate/pull/2515) (open), which adds the field to Substrate's API.

the `fix(substrate): apply a credential header by its request path prefix` commit on `giantswarm`, a `cherry-pick -x` of the upstream branch's commit (`substrate: …` upstream), from [#65](https://github.com/giantswarm/agentgateway-upstream/pull/65) ([#62](https://github.com/giantswarm/agentgateway-upstream/issues/62))

[agentgateway#3985](https://github.com/agentgateway/agentgateway/pull/3985) (merged 2026-10-15 as `38a50f69`) adds `CredentialHeader` field 4 (`actor_jwt`) but no `path_prefix`, so it is the base this change rebases on, not its exit, and the outcome stays to-file (checked 2026-10-20).

2026-10-20, decided on internal issue 38652, comment 6301541135: class 6 fork-only, follower of the Substrate path_prefix row; destination agentgateway/agentgateway, the dataplane half of whichever shape Substrate takes (a prefix comparison in apply_effects under the scheme-keyed entry); first step after Substrate answers; branch `upstream/substrate-egress-path-prefix` @ `efb30d4f`; test `substrate_egress_replaces_only_requested_credentials`; depends on no theme; re-check 2026-11-17; owner unassigned; draft: internal issue 38652, comment 6301547887.
