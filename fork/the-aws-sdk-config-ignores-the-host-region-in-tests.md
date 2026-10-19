---
id: the-aws-sdk-config-ignores-the-host-region-in-tests
fork: agentgateway
theme: test-flakes
category: test
status: carried
upstream:
  outcome: to-file
  refs: []
  exit: file in the test-flake batch
depends_on: []
commit: 11605c48
verified: 2026-10-19
---
# The AWS SDK config ignores the host region in tests

- **Status:** carried
- **Upstream:** to file — prepared in the fork: branch `upstream/aws-sign-test-env-isolation` (`1e8ea861`, on the mirror `main` @ `955d56fa`). Drops at the re-pin onto a commit containing it
- **Our commit:** the `fix(aws): isolate the cached SDK config from the host region in tests` commit, from [giantswarm/agentgateway-upstream#93](https://github.com/giantswarm/agentgateway-upstream/issues/93)

## Change

In test builds the process-wide cached AWS SDK config (`sdk_config` in `crates/agentgateway/src/http/auth/aws.rs`) resolves no ambient region (`AWS_REGION`, `~/.aws/config`, IMDS), and `test_aws_sign_request_no_region_error` in `http/auth/tests.rs` no longer sets `AWS_PROFILE`. Test-only: release builds load the config as before.

## Reason

`test_aws_sign_request_no_region_error` failed intermittently under a parallel `cargo test -p agentgateway --lib` on a machine with a default AWS region: the cached config takes the region of whichever test loads it first, and the test's `AWS_PROFILE=/dev/null` only helped when it won that race, ignored `AWS_REGION` and leaked into concurrent tests. A flaky test in the proxy suite makes the line's `ci-ok` and local proofs unreliable.
