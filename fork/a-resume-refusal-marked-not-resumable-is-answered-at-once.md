# A resume refusal marked not resumable is answered at once

- **Status:** carried
- **Upstream:** not for upstream as is. Upstream Substrate removed the directive's only producer, the `onResume.fromData` golden resolution (agent-substrate/substrate#2105, `6a35150e`), and the Substrate line dropped #80 at its 2026-10-08 re-pin; only giantswarm/substrate `release-1.3` still sends `GOLDEN_SNAPSHOT_UNAVAILABLE` (`internal/ateerrors`), so the patch stays while an installation runs Substrate 1.3 and drops after. The general problem is real on upstream `main` @ `85d1f087` (`retryable_while_parked` in `ingress.rs` parks `FailedPrecondition` for the whole budget), but a Giant Swarm-only ErrorInfo key is no contract; the proper fix is Substrate's codes (a transient conflict answered `ABORTED`, `FAILED_PRECONDITION` kept for refusals that need a state change), raised with agent-substrate first, after which agentgateway, and Substrate's own atenet resumer, stop parking `FailedPrecondition`. Branch [`upstream/substrate-resume-not-resumable`](https://github.com/giantswarm/agentgateway-upstream/tree/upstream/substrate-resume-not-resumable) is not filed (#37742 row 72); agentgateway#3674 merged 2026-10-08 (`73b52e38`) and no longer blocks anything
- **Our commit:** `5e124eb3` ([#29](https://github.com/giantswarm/agentgateway-upstream/pull/29), replayed clean at the 2026-10-05 re-pin)

## Change

Answer a resume refusal the Substrate control plane marks not resumable at once, with the control plane's own status (`http/substrate/ingress.rs`, `proxy/mod.rs`)

## Reason

Substrate refuses to resume an actor whose ActorTemplate lost its golden snapshot (an actor created before its WorkerPool moved to a new worker image) with `FailedPrecondition`, the code it also answers for an actor in transition, which the ingress retries while it parks a request: the caller waited out the parking budget (30 s) and got `UNAVAILABLE` "actor … request timed out", the gRPC code derived from an HTTP 504 ([giantswarm/substrate#79](https://github.com/giantswarm/substrate/issues/79)). The control plane marks such a refusal with the ErrorInfo directive `resumable: "false"` in its `substrate.dev` domain ([giantswarm/substrate#80](https://github.com/giantswarm/substrate/pull/80)); the ingress answers it at once as `ProxyError::SubstrateResumeRefused`: a gRPC caller gets the control plane's code, message and `grpc-status-details-bin`, an HTTP caller the code's HTTP status (400). A refusal without the directive is parked and retried as before
