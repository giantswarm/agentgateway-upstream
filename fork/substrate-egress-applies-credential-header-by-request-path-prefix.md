# The Substrate egress applies a credential header by its request path prefix

- **Status:** carried
- **Upstream:** to file — prepared in the fork: branch [`upstream/substrate-egress-path-prefix`](https://github.com/giantswarm/agentgateway-upstream/tree/upstream/substrate-egress-path-prefix) on upstream `main` @ `50067185`, queued in the team's upstream review list (giantswarm/giantswarm#37742). It follows [agent-substrate/substrate#2515](https://github.com/agent-substrate/substrate/pull/2515) (open), which adds the field to Substrate's API.
- **Our commit:** the `fix(substrate): apply a credential header by its request path prefix` commit on `giantswarm`, a `cherry-pick -x` of the upstream branch's commit (`substrate: …` upstream), from [#65](https://github.com/giantswarm/agentgateway-upstream/pull/65) ([#62](https://github.com/giantswarm/agentgateway-upstream/issues/62))

## Change

`CredentialHeader` in `crates/protos/proto/ateapi.proto` gains `path_prefix` (field 5). `SubstrateEgress::apply_effects` (`crates/agentgateway/src/http/substrate/egress.rs`) applies, per header name (case-insensitive), the `replace_headers` entry with the longest `path_prefix` the request path lies under, the entry without a prefix otherwise, none if neither, and fetches only the chosen entry's credential (`replace_headers_for`). A prefix matches the path without its query at a segment boundary; a path with a dot or empty segment, `..;`, a backslash, or a percent-encoded `.`, `/` or `\` matches no prefix. The same rules as Substrate's `egresspolicy.ReplaceHeadersFor`, which its Envoy egress applies. A policy without prefixes applies as before. Unit tests mirror Substrate's `TestReplaceHeadersFor` and `TestPathPrefixMatches`.

## Reason

The Substrate line carries `path_prefix` (giantswarm/substrate#349, for giantswarm/substrate#344): one host takes `Basic` for Git over HTTPS and `Bearer` for its REST API under `/api/`. Without this change the egress applied every entry for a header, the last one winning on every path, so a path-scoped credential reached every path of the host and the Substrate line's agentgateway e2e failed `TestActorEgressCredentialInjection`.

Drops at the re-pin onto an upstream commit that contains the upstream change.
