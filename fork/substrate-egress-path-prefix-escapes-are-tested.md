---
id: substrate-egress-path-prefix-escapes-are-tested
fork: agentgateway
theme: credential-injection
category: test
status: carried
commits:
  - 'test(substrate): test the path-prefix escape forms of replace_headers_for'
upstream:
  outcome: to-file
  refs: []
  exit: goes with the path_prefix PR
depends_on: [substrate-egress-applies-credential-header-by-request-path-prefix]
verified: 2026-10-19
---
# The Substrate egress path-prefix escape forms are tested

## Change

`replace_headers_ignore_a_prefix_a_path_could_escape` in `crates/agentgateway/src/http/substrate/egress.rs` mirrors Substrate's `TestReplaceHeadersForEscapes` case for case: a path that starts with a credential header's `path_prefix` but could resolve outside it (dot segments, `%2e`/`%2E`, `%2f`/`..%2f`, `%5c`/`%5C`, a doubled slash, `..;` path parameters, a literal backslash, a query repeating the prefix) gets no scoped entry, and the host-wide entry when one exists. Test-only; `replace_headers_for` is unchanged, every case passes as it is.

## Reason

The path-prefix change (`substrate-egress-applies-credential-header-by-request-path-prefix.md`) tested only part of these forms. The behaviour is safe today; the table guards it against a regression that would hand a path-scoped credential to a path outside its prefix.

## History

to file — prepared in the fork: branch [`upstream/substrate-egress-path-prefix-escapes`](https://github.com/giantswarm/agentgateway-upstream/tree/upstream/substrate-egress-path-prefix-escapes) (`2b4cb408`), on top of `upstream/substrate-egress-path-prefix` (`efb30d4f`), so it goes upstream with or after that change. Drops at the re-pin onto a commit containing it

the `test(substrate): test the path-prefix escape forms of replace_headers_for` commit, a `cherry-pick -x` of the upstream branch's commit, from [giantswarm/agentgateway-upstream#97](https://github.com/giantswarm/agentgateway-upstream/issues/97)
