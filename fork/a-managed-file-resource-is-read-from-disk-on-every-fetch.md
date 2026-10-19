---
id: a-managed-file-resource-is-read-from-disk-on-every-fetch
fork: agentgateway
theme: version-pins
category: bugfix
status: carried
commits:
  - 'resource_manager: read a file resource from disk on every managed fetch'
upstream:
  outcome: drops-at-repin
  refs: [agentgateway/agentgateway#3686, agentgateway/agentgateway#3751, agentgateway/agentgateway#3863]
  exit: drops with the row above; rotation test already passes on upstream without it
depends_on: [the-files-a-config-parse-reads-are-managed-dependencies]
verified: 2026-10-19
---
# A managed file resource is read from disk on every fetch

## Change

Read a file resource from disk on every managed fetch (`ResourceManager::fetch_and_wait_normalized` in `crates/agentgateway/src/resource_manager.rs`): a config reload, triggered by any one resource's change, rebuilds every file-backed TLS config from the files' current content instead of the resource cache

## Reason

Substrate's egress and router gateways take their client certificate for the API server from a Kubernetes projected `podCertificate` volume the kubelet rotates every 24 h by swapping the `..data` symlink; on an installation the servicedns bundle's watch reloaded the config before kubelet had written the rotated podidentity bundle, and nothing read that bundle again (it was no managed resource: the next row), so every sandbox CONNECT was refused with `CertificateExpired` until a restart. Reading a local file on a reload is cheap; HTTP resources keep the cache. Reaches managed files only: the Substrate policies' own TLS files became managed with #23

## History

not for upstream: on upstream `main` @ `c7ac602e` the watcher follows kubelet's `..data` symlink swap (this patch's projected-volume rotation test passes there), and a stale cached read during a reload is corrected by the file's own watch event, so the gap that remains upstream is the next row's ([agentgateway#3751](https://github.com/agentgateway/agentgateway/issues/3751)); agentgateway#3686 (merged 2026-10-01 as `5bc6301f`, in the pin since v1.6.0) is complementary and does not replace it. A workaround, not a gap: on upstream `main` @ `85d1f087` a stale cached read is followed by the file's own `refresh_file`, whose notify runs one more reload, and agentgateway#3863 (`0a8476fe`, the next row) also refreshes every newly active file in `retain_resources`, which the line lacks and which the 60 s poll hid. Drops together with the next row at the re-pin onto a commit containing `0a8476fe`, after this patch's projected-volume rotation test passes there without it

`4078f630` (the squash of pull request #20: `resource_manager: read a file resource from disk on every managed fetch (#20)`, replayed clean at the 2026-09-24 and 2026-10-05 re-pins; in release 2.1.1)
