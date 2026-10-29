---
id: a-credential-providers-failure-is-answered-by-its-class
fork: agentgateway
theme: credential-injection
category: feature
status: carried
commits:
  - 'fix(substrate): answer a credential provider''s failure by its class'
upstream:
  outcome: open
  refs: [agent-substrate/substrate#2555, agent-substrate/substrate#2706, agentgateway/agentgateway#3875, agentgateway/agentgateway#2162, agentgateway/agentgateway#3524, agentgateway/agentgateway#4226]
  exit: "agentgateway/agentgateway#3875 merged after agent-substrate/substrate#2706: re-seated without the body sanitizing; agentgateway/agentgateway#4226 merged: the local-500 part dropped; re-check each re-pin"
depends_on: [agent-substrate/substrate#2706, agentgateway/agentgateway#2162]
verified: 2026-10-29
---
# A credential provider's failure is answered by its class

## Change

Answer a credential provider's failure by its class (`http/substrate/egress.rs`, `proxy/mod.rs`, the contract in `protos/proto/credprovider.proto`): `NotFound`, `PermissionDenied`, `FailedPrecondition` and `Unauthenticated` are 403 `credential provider <name> denied: <message>` with the provider's message sanitized (control characters escaped, at most 512 bytes); `Unavailable`, `DeadlineExceeded`, `ResourceExhausted` and an unreachable provider are 503 `credential provider <name> unavailable`; any other status and an unusable secret are 502 `credential provider <name> failed` (`SubstrateEgressFailed`). The full status is logged, never sent

## Reason

the body reaches the actor, and git prints a `text/plain` error body as `remote:` lines: the gateway sent tonic's whole status there (`code: '…'` and, for an outage, the transport `source` with internal hostnames), and it answered 403 for a provider bug, which an agent reads as a permission problem. kagent's caller credentials need the refusal's reason (no grant at the token broker, no turn running) to reach the agent; no status is a 404 or a 400, which clients read as a missing destination or a malformed request

## History

split in three for upstream; drops when each part has landed or been declined. (1) The classification, [agentgateway#3875](https://github.com/agentgateway/agentgateway/pull/3875) (draft, branch `upstream/substrate-egress-errors` @ `f8e75d0f`): the identity check, `fetch_policy` and the credential providers answer `NotFound`, `PermissionDenied` and `FailedPrecondition` 403, `Unavailable`, `DeadlineExceeded` and `ResourceExhausted` 503, any other status 502 (`SubstrateEgressFailed`), and an unusable secret or header value 502; the body keeps `{status}`, as every other error body in the gateway does. Held: agent-substrate/substrate documents today's mapping (`docs/egress-credential-injection.md`: any other code denies with 403, an unusable credential is 503) and its Envoy gateway implements it (`cmd/atenet/internal/router/egress/credentials.go`); agentgateway#3524 aligned to that contract, so the contract changes in Substrate first. Substrate's CONTRIBUTING asks for an issue before a pull request: the issue is [agent-substrate/substrate#2555](https://github.com/agent-substrate/substrate/issues/2555) (opened 2026-10-13; ericcurtin took the Substrate side on 2026-10-15), then the pull request with the atenet half (`61ef1099` on giantswarm/substrate `giantswarm`) and the `FetchSecret` contract comment in agent-substrate/substrate's `credprovider.proto` (not upstream's copy), then #3875 follows it. (2) The status kept out of the actor's body, the refusal's message sanitized (control and format characters escaped, at most 512 bytes): an upstream issue first, since it departs from how the gateway writes error bodies; the implementation is `fix/substrate-egress-errors` @ `692c6340`. (3) The previous row's 500-mapped local errors (`ServiceNotFound`, `BackendDoesNotExist`) answered as gRPC `Unavailable` next to `NoValidBackends`: a separate small pull request when a slot frees, code in `692c6340`. Upstream `main` @ `85d1f087` still answers 403 for every provider status but `Unavailable` and `DeadlineExceeded` and sends the status `Display` as the body. At the re-pin onto `0c62b5a3` this patch is re-seated without `policy_service_unavailable`, which the previous row's drop removes

`971e8a51` ([#39](https://github.com/giantswarm/agentgateway-upstream/pull/39), squash; re-seated at the 2026-10-05 re-pin on #3689's `FetchSecret` span handling, the refusal log kept inside the `map_err`)

[agentgateway#3524](https://github.com/agentgateway/agentgateway/pull/3524) (upstream's alignment to Substrate's provider contract, merged 2026-09-17 as `8dba3989`, already in the pin) is the contract this change departs from, not its exit, so the outcome stays open (checked 2026-10-20).

2026-10-20: the Substrate contract change is the pull request [agent-substrate/substrate#2706](https://github.com/agent-substrate/substrate/pull/2706) (ericcurtin, opened 2026-10-15, "Fixes #2555"), which maps provider `Internal`, `Unknown`, `Unimplemented` and an unusable credential to 502 and `ResourceExhausted` to 503; #3875 follows it, the outcome stays open.

2026-10-20, decided on internal issue 38652, comment 6301541135: part 1 (the status classes) class 5 upstream PR in flight, agentgateway/agentgateway#3875 after agent-substrate/substrate#2706; part 2 (the sanitized body) class 1 drop, not proposed, since #2162 goes the same direction and supersedes it; part 3 (local 500 errors answered as Unavailable) class 5 upstream PR, uncapped; first step the part-3 PR, agentgateway/agentgateway; branch `upstream/substrate-egress-errors` @ `1195ff7f` for #3875, new branch for part 3; test `credential_provider_errors_preserve_availability_semantics` and `substrate_egress_replaces_only_requested_credentials`; depends on no theme; owner unassigned; draft: internal issue 38652, comment 6301547510.

2026-10-26: agentgateway/agentgateway#2162 merged on 2026-10-21 as `be8864a88`, which supersedes part 2 (the sanitized body). Part 1 still waits on #3875 after agent-substrate/substrate#2706, so the outcome stays open.

2026-10-29: part 3 filed as agentgateway/agentgateway#4226 by fiunchinho from `upstream/grpc-local-errors-unavailable` (`96476c07`, +7/-1 in `proxy/mod.rs`): `ServiceNotFound`, `InvalidBackendType` and `BackendDoesNotExist` join the `NoValidBackends` arm of `proxy_error_to_grpc_status` and answer gRPC `Unavailable`; HTTP callers keep 500; the gRPC mapping test table covers the three. The description is human-written, as the project's generative AI policy asks. Part 1 still waits on #3875, so the outcome stays open.
