---
id: a-remote-jwks-is-refetched-on-an-unknown-key-id-local-configuration
fork: agentgateway
theme: jwks
category: bugfix
status: carried
commits:
  - 'fix: refetch remote JWKS on unknown key id before rejecting'
upstream:
  outcome: open
  refs: [agentgateway/agentgateway#2798, agentgateway/agentgateway#3037, giantswarm/agentgateway-upstream#75]
  exit: 'follows the answer on agentgateway#2798; drops at the re-pin onto the upstream fix for local configuration'
depends_on: []
verified: 2026-10-27
---
# A remote JWKS is refetched on an unknown key id, local configuration

## Change

Refetch a remote JWKS on an unknown key id before refusing the token, local configuration (`ResourceManager::refresh_and_wait`, `JwksRefresher` in `http/jwt.rs`): at most one refetch per resource and 10 s

## Reason

after an issuer's signing-key rotation, tokens signed with the new key were refused (`token uses the unknown key`) until the next scheduled JWKS fetch ([#46](https://github.com/giantswarm/agentgateway-upstream/issues/46)); the local-configuration half of the fix

## History

[agentgateway#3037](https://github.com/agentgateway/agentgateway/pull/3037) (a third party's pull request, open on 2026-10-13 with no activity since 2026-08-15) for [agentgateway#2798](https://github.com/agentgateway/agentgateway/issues/2798); falls away at the re-pin onto the first upstream commit containing it

the `fix: refetch remote JWKS on unknown key id before rejecting` commit, cherry-picked with its author's sign-off from agentgateway#3037

agentgateway#3037 was closed unmerged by a maintainer on 2026-10-20, without a review or a comment, together with other stale pull requests that evening. On 2026-10-24 we asked on agentgateway#2798 where the refetch belongs (controller on a kid-miss hint, or the data plane itself); the commit is carried until that answer lands upstream
