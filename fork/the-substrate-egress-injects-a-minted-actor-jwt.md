---
id: the-substrate-egress-injects-a-minted-actor-jwt
fork: agentgateway
theme: credential-injection
category: feature
status: carried
commits:
  - 'feat(substrate): inject an actor JWT minted through Control.MintActorJwt'
  - 'fix(substrate): mint through Substrate''s Control.MintActorJWT method'
  - 'docs(fork): the egress mints through Substrate''s Control.MintActorJWT'
  - 'fix(substrate): mint through Control.MintActorJwt, the name Substrate serves'
upstream:
  outcome: superseded
  refs: [agent-substrate/substrate#1960, agent-substrate/substrate#2135, agent-substrate/substrate#2141, agent-substrate/substrate#2421, agentgateway/agentgateway#3985, giantswarm/substrate#495]
  exit: 're-pin onto #3985'
depends_on: [agent-substrate/substrate#1960]
verified: 2026-10-20
---
# The Substrate egress injects an actor JWT it mints through Control.MintActorJwt

## Change

`crates/protos/proto/ateapi.proto` gains `CredentialHeader.actor_jwt` (field 4, `ActorJWTSource` with `audiences` and `expiration_seconds`) and the `Control.MintActorJwt` call with its messages, named as Substrate names them: gRPC method names are case-sensitive (a test pins the path `/ateapi.Control/MintActorJwt` on the generated server). `SubstrateEgress::apply_effects` (`crates/agentgateway/src/http/substrate/egress.rs`) injects, for an `actor_jwt` entry whose header the request carries, `<prefix><token>` minted through `MintActorJwt` on the policy's own control API backend (`actor_jwt`, `mint_actor_jwt`), with no credential provider. Tokens are cached per actor name, sorted audiences and lifetime and reused until a third of their lifetime remains (`actor_jwt_refresh_at`); concurrent misses share one mint. A failed mint denies the request: 403 when the actor is gone (`NotFound`), 503 when the control API is unavailable or times out, 500 otherwise (`ProxyError::SubstrateEgressInternal`), as Substrate's `mapActorJWTError`. On the line, "unavailable" is `policy_service_unavailable`, which also reads an unreachable service as unavailable.

## Reason

Upstream Substrate's egress policy carries an `actor_jwt` source (Substrate 1.9.1 on the line). Without this change the agentgateway egress read every entry as an `ate-secret://` URI and denied an `actor_jwt` entry with `403 invalid credential URI: `, which blocks the per-Session caller credential of giantswarm/kagent-upstream#297. The Substrate line pins the release that carries it (giantswarm/substrate#419).

## History

superseded by [agentgateway#3985](https://github.com/agentgateway/agentgateway/pull/3985) (keithmattix, merged 2026-10-15, `38a50f69`): `crates/protos/proto/ateapi.proto` gains `CredentialHeader.actor_jwt` (field 4, `ActorJwtSource`) and `Control.MintActorJwt`, and `http/substrate/egress.rs` mints and injects the token. Upstream calls `/ateapi.Control/MintActorJwt`, the name agent-substrate/substrate `main` declares (`pkg/proto/ateapipb/ateapi.proto`, `rpc MintActorJwt`); the line's `MintActorJWT` follows giantswarm/substrate `giantswarm` (`rpc MintActorJWT`), which differs. Drops at the re-pin onto a commit containing `38a50f69` once the Substrate line serves `MintActorJwt`. Prepared in the fork before #3985: branch [`upstream/substrate-egress-actor-jwt-v2`](https://github.com/giantswarm/agentgateway-upstream/tree/upstream/substrate-egress-actor-jwt-v2) on upstream `main` @ `e523afba` (commit `49a49627`, the injection with the method name Substrate serves; it replaces the first branch `upstream/substrate-egress-actor-jwt`). It follows Substrate's actor-JWT injection on its Envoy egress ([agent-substrate/substrate#1960](https://github.com/agent-substrate/substrate/pull/1960), [#2135](https://github.com/agent-substrate/substrate/pull/2135), [#2141](https://github.com/agent-substrate/substrate/pull/2141), [#2421](https://github.com/agent-substrate/substrate/pull/2421)).

the `feat(substrate): inject an actor JWT minted through Control.MintActorJwt` commit on `giantswarm`, a `cherry-pick -x` of the first branch's commit `3902417a`, from [#79](https://github.com/giantswarm/agentgateway-upstream/issues/79), and `fix(substrate): mint through Substrate's Control.MintActorJWT method` ([#82](https://github.com/giantswarm/agentgateway-upstream/pull/82)), the rest of `49a49627`

[giantswarm/substrate#495](https://github.com/giantswarm/substrate/pull/495) (merged 2026-10-20, in `v1.17.0-rc.1`) renames the Substrate line's RPC to `MintActorJwt`, as agent-substrate/substrate#2520 named it on `main`. From the Substrate release that carries it, ate-api answers this line's `/ateapi.Control/MintActorJWT` with `Unimplemented`, which the egress denies with 500, so the egress's caller moves to `MintActorJwt` in the same release window and the Substrate chart's `images.agentgateway` moves to that release.

The `fix(substrate): mint through Control.MintActorJwt, the name Substrate serves` commit names the call `MintActorJwt` again, the name upstream and the Substrate line from 1.17 serve, so the line and #3985 call the same path.
