---
id: substrate-egress-path-prefix-admits-an-encoded-slash
fork: agentgateway
theme: credential-injection
category: bugfix
status: carried
upstream:
  outcome: to-file
  refs: []
  exit: goes with the path_prefix PR
depends_on: [substrate-egress-applies-credential-header-by-request-path-prefix, substrate-egress-path-prefix-escapes-are-tested, giantswarm/substrate#481]
commit: ""
verified: 2026-10-19
---
# A path-prefixed credential header applies to a path with an encoded slash

- **Status:** carried
- **Upstream:** to file — prepared in the fork: branch [`upstream/substrate-egress-encoded-slash`](https://github.com/giantswarm/agentgateway-upstream/tree/upstream/substrate-egress-encoded-slash) (`3fb47a60`), on top of `upstream/substrate-egress-path-prefix-escapes` (`2b4cb408`), so it goes upstream with or after the path-prefix change. Drops at the re-pin onto a commit containing it
- **Our commit:** the `fix(substrate): apply a path-prefixed credential header to a path with an encoded slash` commit on `giantswarm`, a `cherry-pick -x` of the upstream branch's commit, from [giantswarm/agentgateway-upstream#105](https://github.com/giantswarm/agentgateway-upstream/issues/105)

## Change

`canonical_path` in `crates/agentgateway/src/http/substrate/egress.rs` accepts a percent-encoded `/` when the path has no dot or empty segment read either way, `%2F` taken as a separator as well; a percent-encoded `.` or `\` still makes a path non-canonical. A prefix still compares byte for byte, so an encoded slash never moves a path under a prefix it is not under as written. The `path_prefix` comment in `crates/protos/proto/ateapi.proto` states the rule. `replace_headers_apply_a_prefix_to_an_encoded_slash_under_it` covers GitLab's encoded project, file and branch paths; `replace_headers_ignore_a_prefix_a_path_could_escape` gains the encoded-slash escape forms; `substrate_egress_replaces_only_requested_credentials` in `tests/tests/substrate.rs` scopes its credential to a path prefix and sends `%2F` and `%252F`, which the origin receives as sent, with the credential.

## Reason

GitLab addresses a project by its URL-encoded path (`/api/v4/projects/<group>%2F<project>`), and the workspace credential for its API is scoped to `/api/v4/`. With every `%2F` path non-canonical, such a request reached GitLab without the person's token and answered `404 Project Not Found`, so `glab api projects/<group>%2F<project>` and `glab mr create` failed from a Session. Substrate's `egresspolicy.ReplaceHeadersFor` follows the same rule ([giantswarm/substrate#481](https://github.com/giantswarm/substrate/issues/481)).
