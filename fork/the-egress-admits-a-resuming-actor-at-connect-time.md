# The egress admits a resuming actor at CONNECT time

- **Status:** carried
- **Upstream:** merged as [agentgateway#3882](https://github.com/agentgateway/agentgateway/pull/3882) (`9d36620d`, 2026-10-09): `egress_actor_resolution.rs` admits `Running | Resuming`, with integration tests in `tests/tests/substrate.rs`. Our draft agentgateway#3876 and branch `upstream/substrate-egress-resuming` are superseded by it. Drops at the re-pin onto a commit containing `9d36620d` (#37742 row 8)
- **Our commit:** `52f1c49a` (the `substrate: admit a resuming actor's egress at CONNECT time` commit, replayed clean at the 2026-09-22 and 2026-09-24 re-pins and re-seated by hand at the 2026-10-05 re-pin onto v1.6.0, where agentgateway#3677 had rewritten `egress_actor_resolution.rs`: the patch's UID comparison left with the certificate extension #3677 removed, the placement check stands on `GetActor`'s state as before; on the v1.5.0 base it was the same change in `egress.rs`, pull request #4 — at the 2026-09-14 re-pin the prepared main-shaped commit `7771e400` of `upstream/substrate-egress-resuming` replaced it, the check having moved to `egress_actor_resolution.rs` in #3318)

## Change

Admit a `RESUMING` actor at CONNECT time next to `RUNNING` (SUSPENDED, PAUSED, CRASHED and DELETING stay refused; the denial names the state)

## Reason

Substrate commits `RUNNING` only after the workload served readyz; a workload that fetches what it needs to become ready — kagent's Go ADK and Claude harnesses materialise git skills before readyz — was refused (`403 Forbidden: actor is not running`) and its ActorTemplate never got its golden snapshot ([#37742](https://github.com/giantswarm/giantswarm/issues/37742) row 8; acceptance test `agentlab skills-test`, [agentlab#137](https://github.com/giantswarm/agentlab/issues/137)). Counterpart of the Substrate line's [giantswarm/substrate#4](https://github.com/giantswarm/substrate/pull/4) (ateom arms the tunnel before the first container starts, atenet's `ext_proc` admits `RESUMING`)
