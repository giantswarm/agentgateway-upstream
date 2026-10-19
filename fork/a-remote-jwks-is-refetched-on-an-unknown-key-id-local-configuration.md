---
id: a-remote-jwks-is-refetched-on-an-unknown-key-id-local-configuration
fork: agentgateway
theme: jwks
category: bugfix
status: carried
upstream:
  outcome: open
  refs: [agentgateway/agentgateway#2798, agentgateway/agentgateway#3037, giantswarm/agentgateway-upstream#75]
  exit: 'lands or dies with #3037'
depends_on: []
commit: 14e5f1a8
verified: 2026-10-19
---
# A remote JWKS is refetched on an unknown key id, local configuration

- **Status:** carried
- **Upstream:** [agentgateway#3037](https://github.com/agentgateway/agentgateway/pull/3037) (a third party's pull request, open on 2026-10-13 with no activity since 2026-08-15) for [agentgateway#2798](https://github.com/agentgateway/agentgateway/issues/2798); falls away at the re-pin onto the first upstream commit containing it
- **Our commit:** the `fix: refetch remote JWKS on unknown key id before rejecting` commit, cherry-picked with its author's sign-off from agentgateway#3037

## Change

Refetch a remote JWKS on an unknown key id before refusing the token, local configuration (`ResourceManager::refresh_and_wait`, `JwksRefresher` in `http/jwt.rs`): at most one refetch per resource and 10 s

## Reason

after an issuer's signing-key rotation, tokens signed with the new key were refused (`token uses the unknown key`) until the next scheduled JWKS fetch ([#46](https://github.com/giantswarm/agentgateway-upstream/issues/46)); the local-configuration half of the fix
