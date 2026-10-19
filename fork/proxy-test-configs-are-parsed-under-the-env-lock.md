---
id: proxy-test-configs-are-parsed-under-the-env-lock
fork: agentgateway
theme: test-flakes
category: test
status: carried
upstream:
  outcome: to-file
  refs: []
  exit: file in the test-flake batch
depends_on: []
commit: c0144ae0
verified: 2026-10-19
---
# Proxy test configs are parsed under the env lock

- **Status:** carried
- **Upstream:** to file — prepared in the fork: branch `upstream/oauth-test-xds-env-lock` (`3dfcc6b0`, on the mirror `main` @ `955d56fa`). Drops at the re-pin onto a commit containing it
- **Our commit:** the `fix(test): parse proxy test configs under the env lock` commit, from [giantswarm/agentgateway-upstream#95](https://github.com/giantswarm/agentgateway-upstream/issues/95)

## Change

`setup_proxy_test` and `setup_llm_named_provider_mock` in `crates/agentgateway/src/test_helpers/proxymock.rs` parse their config through `parse_test_config`, which holds the shared env lock (`crate::config::lock_env_for_tests_async`, taken with `block_on` so it works in sync and async tests). Test-only: outside `cfg(test)` the helper parses as before.

## Reason

`http::auth::oauth::tests::{sends_form_params, rejects_mismatched_issued_token_type}` failed in about 1 of 20 parallel `cargo test -p agentgateway --lib` runs with "NAMESPACE/config.namespace is required when XDS is configured": `xds_headers_are_loaded_from_environment` in `config.rs` sets `XDS_ADDRESS`, `NAMESPACE` and `GATEWAY` one at a time under the env lock, and a proxy test that parsed its config in between, without the lock, saw an XDS address without a namespace. A flaky test in the proxy suite makes the line's `ci-ok` and local proofs unreliable.
