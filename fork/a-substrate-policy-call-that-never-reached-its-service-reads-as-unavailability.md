---
id: a-substrate-policy-call-that-never-reached-its-service-reads-as-unavailability
fork: agentgateway
theme: credential-injection
category: bugfix
status: carried
commits:
  - 'fix(substrate): an unreachable policy service is unavailable, not denied'
upstream:
  outcome: merged
  refs: [agentgateway/agentgateway#3752, agentgateway/agentgateway#3793]
  exit: drops at re-pin; 500-mapped local errors (ServiceNotFound) still read as Unknown, later PR
depends_on: []
verified: 2026-10-19
---
# A Substrate policy call that never reached its service reads as unavailability

## Change

Read a Substrate policy call that never reached its service as unavailability (`http/substrate/mod.rs`, `egress.rs`, `egress_actor_resolution.rs`): a status tonic builds from the channel's own `ProxyError` (a refused connect, a failed DNS lookup, no endpoint) is code `Unknown`, which the actor identity check, the egress policy and the credential providers answered as a denial (403 `actor egress policy denied … upstream call failed: Connect: Connection refused`); it is now `SubstrateEgressUnavailable` (503), and the service's own answers keep their meaning

## Reason

the control plane behind the headless `api` Service had an endpoint that served nothing, so one policy call in three was refused at connect; the egress told a worker's `git fetch` it was forbidden, and the lab chased a network policy and the route to github.com instead of the control plane ([giantswarm/agentlab#336](https://github.com/giantswarm/agentlab/issues/336))

## History

to file: the upstream-shaped commit with DCO sign-off is branch [`upstream/substrate-policy-unreachable`](https://github.com/giantswarm/agentgateway-upstream/tree/upstream/substrate-policy-unreachable) here; [agentgateway#3752](https://github.com/agentgateway/agentgateway/issues/3752) (issue, 2026-10-02): upstream `main` @ `c7ac602e` answers an unreachable service with 403 on egress and 500 on ingress, reproduced with integration tests; the cause is `GrpcReferenceChannel` returning the local `ProxyError`, which tonic reads as `UNKNOWN`, the PR [agentgateway#3793](https://github.com/agentgateway/agentgateway/pull/3793) (branch `upstream/grpc-channel-status`) merged 2026-10-08 as `0c62b5a3` and #3752 is closed: the channel-level fix, `GrpcReferenceChannel` answers a local `ProxyError` as a gRPC status mapped from its HTTP status, so the identity check, `fetch_policy` and the credential providers answer 503 and ingress retries while parked; branch `upstream/substrate-policy-unreachable` is superseded. Drops at the re-pin onto a commit containing `0c62b5a3`. One gap this patch covered stays upstream: local errors that map to 500 (`ServiceNotFound`, `InvalidBackendType`, `BackendDoesNotExist`) still read as `Unknown`, so a policy Service missing from discovery is answered 403 denied; it is filed with the next row

the `fix(substrate): an unreachable policy service is unavailable, not denied` commit of pull request [#35](https://github.com/giantswarm/agentgateway-upstream/pull/35) (rebase), `2f1ea6dc` since the 2026-10-05 re-pin, where it was re-seated on #3689 (`c4143926`, in v1.6.0: the egress policy call moved into `fetch_policy`, the resolver's `tonic::Code` import left with the extension parsing)
