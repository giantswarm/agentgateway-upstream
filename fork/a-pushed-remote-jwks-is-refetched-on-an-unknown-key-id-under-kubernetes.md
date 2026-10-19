---
id: a-pushed-remote-jwks-is-refetched-on-an-unknown-key-id-under-kubernetes
fork: agentgateway
theme: jwks
category: feature
status: carried
upstream:
  outcome: fork-only
  refs: [agentgateway/agentgateway#2798, agentgateway/agentgateway#3924, agentgateway/agentgateway#3936]
  exit: file draft E issue; no uptake in 4 weeks then drop the RPC and rely on Dex key overlap
depends_on: []
commit: ""
verified: 2026-10-19
---
# A pushed remote JWKS is refetched on an unknown key id under Kubernetes

- **Status:** carried
- **Upstream:** fork-only for now. Upstream's maintainer answered on agentgateway#2798 that an eager fetch "is not easy on kubernetes as the fetch is done by the control plane"; on upstream `main` @ `85d1f087` the controller refetches on a fixed TTL (`TTLForRemote`, 5 minutes unless `cacheDuration` is set, which the CRD keeps at 5 minutes or more) and ignores the response's `Cache-Control`. The upstream path starts with an issue: Dex sends a fixed `Cache-Control: max-age=600` on `/keys` (`server/discovery/discovery.go`) and signs with a new key in the same update that publishes it (`server/signer/rotation.go`), so honouring `Cache-Control` would not close the gap for Dex and only a refetch on an unknown key id does. The issue proposes a lower `cacheDuration` minimum as the small step and asks whether the maintainers want the refetch under Kubernetes; the RPC is offered only then, with the refresh scoped to the caller's own policies and the logged key id truncated. On 2026-10-13 the issue is not filed, and upstream has changed the same code since the pin (agentgateway#3924 `1c282481`, #3936 `ace4cd80`): a trial merge onto upstream `main` @ `c0354ea4` conflicts in `controller/pkg/agentgateway/jwks/fetcher.go` and `api/resource.pb.go`
- **Our commit:** the `fix: refetch a pushed remote JWKS on an unknown key id under Kubernetes`, `fix: answer a held JWKS that has the asked kid without refetching`, `fix(jwt): await the JWKS refresh in flight for an unknown key id` and `fix(jwks): answer a refresh request after the on-demand fetch in flight` commits, on top of the previous row (a replica that does not hold the leader lease answers without persisting; it answers a held keyset that has the asked key id without a fetch; the requests that meet the key id while the refresh runs await its answer, in the data plane and in the controller, [#72](https://github.com/giantswarm/agentgateway-upstream/issues/72)); offered upstream as one change with the RPC

## Change

Refetch a pushed remote JWKS on an unknown key id under Kubernetes (`JWTProvider.remote_jwks_key` and the `JwksRefresh` service in `resource.proto`; the controller's `jwks.RefreshService` and `Fetcher.RefreshNow`, the data plane's `control::jwks::ControlPlaneJwks` with `RefreshCalls`, one shared call per key and interval that every request meeting the key id awaits)

## Reason

under Kubernetes the controller fetches the JWKS every 5 minutes and pushes the keys inline, so the previous row never applies: a Dex key rotation left fresh tokens refused for up to 5 minutes on the platform's gateway ([#46](https://github.com/giantswarm/agentgateway-upstream/issues/46)). The data plane asks the controller over its xDS connection (same authentication); the controller fetches at most once per key and 10 s, pushes a changed set through the normal policy update and answers the set so the asking request validates at once. A request that meets the unknown key id while that call is in flight awaits it (at most 3 s) instead of being refused, and the controller answers a refresh request that arrives during its fetch with what the fetch brings; before, every request of the controller's round trip was refused ([#72](https://github.com/giantswarm/agentgateway-upstream/issues/72))
