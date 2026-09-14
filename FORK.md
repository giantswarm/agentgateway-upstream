# The Giant Swarm line of agentgateway

This repository is Team Bumblebee's fork of [agentgateway/agentgateway](https://github.com/agentgateway/agentgateway).
The Giant Swarm Agent Platform runs agentgateway in two places — as the platform's gateway (the controller and the
data-plane proxy the [`giantswarm/agentgateway`](https://github.com/giantswarm/agentgateway) packaging chart
deploys) and inside Agent Substrate (`atenet-router` and `atenet-egress` of the
[`giantswarm/substrate`](https://github.com/giantswarm/substrate) line) — and this fork is where the agentgateway
they run is pinned, built, scanned and published. It answers one question from one place: **which agentgateway are
we running, and why does it differ from upstream?**

Tracking (this fork has issues disabled): the line [giantswarm/giantswarm#37758](https://github.com/giantswarm/giantswarm/issues/37758),
upstream engagement [giantswarm/giantswarm#37742](https://github.com/giantswarm/giantswarm/issues/37742), the sibling
lines built the same way — kagent [giantswarm/giantswarm#37010](https://github.com/giantswarm/giantswarm/issues/37010)
(`giantswarm/kagent-upstream`) and Substrate [giantswarm/giantswarm#37757](https://github.com/giantswarm/giantswarm/issues/37757)
(`giantswarm/substrate`) — and the epic [giantswarm/giantswarm#37705](https://github.com/giantswarm/giantswarm/issues/37705).

## Branches

| Branch | What it is | Who moves it |
|---|---|---|
| `main` | A pure mirror of upstream `main`, fast-forwarded. Never edited, never the target of a pull request. | the sync workflow (as the HeraldBot App); the `protect-main` ruleset admits the App and repository admins (manual repair) and nobody else |
| `giantswarm` (default) | **The line**: the upstream release tag the platform runs ("the pin") + cherry-picked upstream fixes + this fork's own files. Every change of the fork's own files is a pull request against it. | pull requests (`ci-ok` required); the sync workflow and repository admins may force-push it for a re-pin |
| `fork/<topic>` | pull-request branches against `giantswarm` | anyone in the team |
| `sync/<date>-<pin>` | hand-over branches the sync workflow opens when a re-pin conflicts | the sync workflow; a human finishes them |

The line was bootstrapped on 2026-09-11 by pushing the tag commit of the pin to `giantswarm` by hand (the one push to
the branch that was not a pull request or a re-pin), then adding the fork's files through pull request #1.

## Pin

| | |
|---|---|
| Upstream commit | **`main` @ `c1d24607`** (2026-09-14, "build(deps): bump the cargo-weekly group with 18 updates (#3454)"; 118 commits past v1.5.0). A `main` commit, not a release tag: upstream has released nothing since v1.5.0 (2026-08-27), and the Substrate line's pin needs what `main` carries (below). The line returns to a release tag at the first upstream release that contains `9f9744cf` (v1.6.0 by upstream's cadence). |
| Why this one | The Substrate line (`giantswarm/substrate`) is pinned to kagent-dev/substrate **v0.0.29**, whose chart declares the egress actor check as the frontend policy `substrateEgressActorResolution` of agentgateway#3318 and whose `atenet-router` speaks the substrate ingress protocol of agentgateway#3409/#3410/#3428 — upstream kagent-dev/substrate pins a nightly of upstream commit `9f9744cf` for it ([kagent-dev/substrate#28](https://github.com/kagent-dev/substrate/pull/28)). Any `main` commit ≥ `9f9744cf` serves; the line takes `main`'s head of the day the three lines moved together (agentgateway → Substrate → kagent, giantswarm/giantswarm#37742). The packaging chart `giantswarm/agentgateway` follows the same release for the platform's controller and data plane (see "Consumers"). |
| When it moves | with the consumers, proven in agentlab first — see "Re-pin". Not on a schedule. Previous pin: v1.5.0 (2026-09-10 → 2026-09-14, releases `v1.5.1-gs.1`–`gs.3`). |
| Derived how | `git describe --tags --abbrev=0 --match 'v[0-9]*' --exclude '*-*' giantswarm` with upstream's tags fetched names the nearest upstream release **below** the pin — v1.5.0 — so the dev base and the release base stay `1.5.1` while the pin is a `main` commit; the line's own tags carry a pre-release suffix and are excluded. The pin itself is `git merge-base giantswarm main` (the mirror). The workflows compute both, nothing records them twice. |

### Convergence with the Substrate line

Upstream kagent-dev/substrate moved its router image to `ghcr.io/agentgateway/agentgateway:v0.0.0-alpha.9f9744cf`
([kagent-dev/substrate#28](https://github.com/kagent-dev/substrate/pull/28), 2026-09-10, in v0.0.28 and v0.0.29) — a
nightly of upstream agentgateway commit `9f9744cf` ("substrate: Fix custom port (#3428)", 117 commits past v1.5.0).
Between v1.5.0 and it upstream changed the substrate protocol: #3237 (authorize actor egress at CONNECT time), #3289,
#3333 (router metrics), #3335, #3318 (egress policy, `substrateEgressActorResolution`), **#3409 (the new substrate
ingress header)**, #3428. The Substrate line's re-pin onto v0.0.29 (2026-09-14) therefore moved this line onto `main`
≥ `9f9744cf` first, and the Substrate chart pins this line's `v1.5.1-gs.4` for `atenet-router` and `atenet-egress`.
One branch serves both consumers: the packaging chart runs the same release for the platform's controller and data
plane (its vendored chart stays upstream's v1.5.0 — the chart did not change between v1.5.0 and the pin in a way the
platform's values touch; the drift is recorded in the packaging chart's own `FORK.md`/README and closes at the first
upstream release the line moves onto).

## Carried patches

Everything on `giantswarm` that is not in the pin (`git log main..giantswarm`, the mirror `main` being the pin):

| Patch | Purpose | Fork commit | Upstream |
|---|---|---|---|
| Admit a `RESUMING` actor at CONNECT time next to `RUNNING` (SUSPENDED, PAUSED, CRASHED and DELETING stay refused; the denial names the state) | Substrate commits `RUNNING` only after the workload served readyz; a workload that fetches what it needs to become ready — kagent's Go ADK and Claude harnesses materialise git skills before readyz — was refused (`403 Forbidden: actor is not running`) and its ActorTemplate never got its golden snapshot ([#37742](https://github.com/giantswarm/giantswarm/issues/37742) row 8; acceptance test `agentlab skills-test`, [agentlab#137](https://github.com/giantswarm/agentlab/issues/137)). Counterpart of the Substrate line's [giantswarm/substrate#4](https://github.com/giantswarm/substrate/pull/4) (ateom arms the tunnel before the first container starts, atenet's `ext_proc` admits `RESUMING`) | `9a4c4731` (the `substrate: admit a resuming actor's egress at CONNECT time` commit; on the v1.5.0 base it was the same change in `egress.rs`, pull request #4 — at the 2026-09-14 re-pin the prepared main-shaped commit `7771e400` of `upstream/substrate-egress-resuming` replaced it, the check having moved to `egress_actor_resolution.rs` in #3318) | to file: the upstream-shaped patch is branch [`upstream/substrate-egress-resuming`](https://github.com/giantswarm/agentgateway-upstream/tree/upstream/substrate-egress-resuming) here (`7771e400`, on the mirror `main` @ `fddff503`; the carried commit is this commit replayed); a team member opens the agentgateway pull request with DCO sign-off once #37742 has reviewed it (row 8) |
| Translate a `GRPCRoute` service-only method match to a path prefix and honour `type: RegularExpression` (`CreateAgwGRPCPathMatch` in the controller's translator): a service-only match became `PathMatch_Exact "/<service>/"`, a path no `/Service/Method` request has, so the rule never matched; a `RegularExpression` match with a service was emitted as an exact match on the regex text | the platform routes a gRPC API through a `GRPCRoute` that matches its services (so a new RPC needs no route change) — with the defect every call fell through to the catch-all `HTTPRoute` behind it (found on agentgateway v1.5.0 controller and proxy, identical on `main` @ the pin); until the fix is on the line the chart enumerates every service+method pair as exact matches | `8990e17c` (the `fix(controller): translate GRPCRoute service-only and RegularExpression method matches` commit of pull request #6, replayed clean at the 2026-09-14 re-pin) | to file: no upstream issue or pull request covers it (searched 2026-09-11); the upstream-shaped patch is branch [`upstream/grpcroute-method-match-translation`](https://github.com/giantswarm/agentgateway-upstream/tree/upstream/grpcroute-method-match-translation) here (on the mirror `main`); a team member opens the agentgateway pull request with DCO sign-off once #37742 has reviewed it (row 25). Falls away at the re-pin onto the first release containing it |
| Fork infrastructure: this file, the README pointer, `CODEOWNERS`, `.github/workflows/publish.yaml`, `.github/workflows/sync-upstream.yaml`, `.trivyignore`; `pull_request.yml` on `giantswarm` and on `sync/**` pushes (a re-pin candidate is a rebased branch, so a pull request from it runs no `pull_request` workflow — the push runs `ci-ok`) with GitHub-hosted runners, Linux lanes only, `govulncheck` and `ci-ok`; `release.yml` without its tag trigger | the line's CI, publishing and sync | the `giantswarm` branch history | not for upstream |

Two patches change agentgateway's behaviour beyond upstream `main` at the pin: the admission of a resuming actor and
the `GRPCRoute` method-match translation, both written for upstream and leaving at the first upstream commit that
carries them. Dropped at the 2026-09-14 re-pin because upstream had merged them: the `cherry-pick -x` of agentgateway#3237
(the CONNECT-time actor authorization, upstream `8cbb254d`) and the grpc/x/crypto dependency bumps (upstream #3295 and
#3303). Giant Swarm specific configuration lives elsewhere: the gateway's values, routes and policies in the
[agent-platform](https://github.com/giantswarm/agent-platform) meta and connectivity charts, the packaging in
`giantswarm/agentgateway`, the Substrate router's agentgateway configuration in the Substrate chart.

## Re-pin

The re-pin moves the line onto a new upstream commit — a release tag, or a `main` commit while upstream has no
release with what the consumers need — and replays the carried patches; a patch upstream has merged falls away by
itself (`git rebase` drops already-applied patches). It is the one sanctioned rewrite of `giantswarm`.

1. Decide the pin with the consumers: the packaging chart's vendored chart version and the Substrate line's router
   protocol (see "Convergence") must both be served by it. The three lines move in one order — this line first, then
   the Substrate line (its chart pins this line's release), then the kagent line (its `go.mod` pins Substrate).
2. Run **Actions → sync-upstream → Run workflow** with `pin` = the tag or commit (for example `v1.6.0`). The workflow
   mirrors `main`, rebases the carried patches onto the pin, builds the controller as a smoke, and force-pushes
   `giantswarm`. The push runs upstream's suite (`pull_request.yml`) and `publish` builds the dev build.
   - On a conflict it pushes `sync/<date>-<pin>` (the new pin + the patches that applied before the conflict) and
     opens a pull request that names the conflicting patch and the ones behind it. Finish it by hand: check the
     branch out, `git cherry-pick -x` the rest, resolve, test, push the branch (`pull_request.yml` runs `ci-ok` on a
     `sync/**` push — the pull request itself runs nothing, a rebased branch has no merge commit), then
     `git push --force-with-lease=refs/heads/giantswarm origin HEAD:giantswarm` and close the pull request. **Do not
     merge it** — the line is a rebased branch; a merge would fold the old pin back in.
   - `dry_run: true` does everything except the pushes; the run summary shows the outcome.
3. Update this file (pin, carried patches) in a pull request, and the agentgateway rows of #37742.
4. Tag a release (`vX.Y.Z-gs.N`), move the consumers to it (see "Consumers"), prove it in agentlab — for a move the
   Substrate line depends on, the release is cut once `ci-ok` is green on the exact head (the Substrate chart's
   publish resolves the release image by digest), and the agentlab proof runs on the three lines' builds together
   before the Substrate and kagent releases follow.

The 2026-09-14 re-pin (v1.5.0 → `main` @ `c1d24607`) by hand, in a scratch worktree: `git rebase --onto upstream/main
v1.5.0` dropped the deps bump and the #3237 cherry-pick (`git rebase --skip` on their `go.mod`/`schema` conflicts),
the resuming admission conflicted in `egress.rs` and was replaced by `git cherry-pick 7771e400` (the prepared
main-shaped commit), the `GRPCRoute` fix replayed clean; `go build ./...` and `cargo fmt --check` locally, the Rust
lanes in CI on the `sync/20260914-main-c1d24607` push; the force-push by a repository admin (a bypass actor of the
`giantswarm` ruleset). `schema/config.{json,md}` needed no regeneration (the remaining Rust patch touches no config type).

The weekly run (Mondays 05:41 UTC) does not re-pin: it mirrors `main` and **probes** whether the carried patches
still rebase onto upstream `main`, naming the first patch that would conflict in the run summary, so the next
re-pin is never a surprise.

**Identity of the automation.** The workflow pushes as the org's **HeraldBot GitHub App** — a token minted per run
from the org secrets `HERALD_CLIENT_ID` / `HERALD_APP_KEY` (`actions/create-github-app-token`); the App is installed
on every org repository with contents and workflows write access and is a bypass actor (`Integration`) of both
rulesets. Why an App and not the org's machine-account token: GitHub refuses a push from a personal access token
that creates or changes a file under `.github/workflows/` unless the token carries the `workflow` scope, and upstream
`main` — hence every mirror and every re-pin — carries upstream's workflow files. A push with the workflow's own
`GITHUB_TOKEN` would not do either: it triggers no other workflow, and the push to `giantswarm` is what publishes the
dev build. The mirror push does start the workflows upstream's own files define on `main` (they ask for Blacksmith
runners the org does not have); the sync cancels them.

Manual equivalent (a workstation, upstream as a remote):

```sh
git fetch upstream main 'refs/tags/v*:refs/tags/v*'
git checkout giantswarm
git rebase --onto v1.6.0 "$(git merge-base giantswarm upstream/main)"   # new pin, old pin (a tag or a main commit)
go build ./... && make lint
git push origin HEAD:sync/$(date -u +%Y%m%d)-v1.6.0                        # ci-ok on the candidate
git push --force-with-lease=refs/heads/giantswarm origin HEAD:giantswarm
```

## Publishing

`publish.yaml` publishes to `ghcr.io/giantswarm/agentgateway-upstream` on every push to `giantswarm` and on every
`v*` tag; nothing is ever pushed by hand. It is upstream's `release.yml` without the Blacksmith runners, the GitHub
release and the binaries.

| Artifact | Name |
|---|---|
| Data-plane proxy | `ghcr.io/giantswarm/agentgateway-upstream/agentgateway:<tag>` — linux/amd64 + linux/arm64 from the root `Dockerfile` (Rust 1.98 on `chainguard/glibc-dynamic`, the UI embedded), each platform built natively on its runner and merged into one manifest list; cosign-signed (keyless) |
| Controller | `ghcr.io/giantswarm/agentgateway-upstream/controller:<tag>` — the Go binary per platform on `chainguard/static`; cosign-signed, with upstream's OpenVEX attestation |
| Charts | `oci://ghcr.io/giantswarm/agentgateway-upstream/charts/{agentgateway,agentgateway-crds,agentgateway-standalone}:<version>` (and `:<tag>`, as upstream pushes both) — upstream's charts with `image.registry` stamped to this registry; `appVersion` = the image tag |

Not published from here: the `agentgateway` and `agctl` binaries, the Windows image, the s390x image (use upstream's
release for those).

**Versions.** Image tags keep upstream's **`v` prefix** (`v1.5.0` is what the packaging chart's `appVersion`, the
connectivity chart's `proxy.image.tag` and the retagger rules carry); chart versions are the bare semver. The sibling
Substrate and kagent lines tag their images **without** the `v` (ko's convention) — two deliberate choices, do not "fix"
one to match the other.

- Dev build, on every push to `giantswarm`: `<next upstream patch>-dev.giantswarm.<YYYY-MM-DD>.<HH-MM-SS>.h<sha7>`
  (for the pin v1.5.0: images `v1.5.1-dev.giantswarm.…`, charts `1.5.1-dev.giantswarm.…`), the schema the sibling
  lines use — base = the pin's patch + 1, branch lowercased to `[a-z0-9-]`, committer date in UTC, so a rebuild of the
  same commit yields the same version and versions sort chronologically within the branch. A Flux consumer of the
  channel uses `semverFilter: ".*-dev\.giantswarm\..*"`; exact pins name the full string.
- Release, on a tag `vX.Y.Z-gs.N` where `X.Y.Z` is upstream's **next** version (the dev base) and `N` counts the
  line's releases of that pin: `v1.5.1-gs.1`. Ordering by semver: `1.5.1-dev.… < 1.5.1-gs.1 < 1.5.1`, so a dev build
  never outranks a release, a fork release never outranks the upstream version it anticipates, and the switch to an
  upstream tag one day is a range change, not a rename. The fleet consumes releases only (see "Consumers").
- `workflow_dispatch` with a `version` input publishes that string (for a one-off).

**Digests.** Every run writes an `Images`/`Charts` table with the digest of each pushed artifact to its summary and
uploads the image references as the `ref-agentgateway` / `ref-controller` artifacts; consumers pin by tag and verify
by digest from there. Release digests are recorded here:

| Release | Pin | Images and charts |
|---|---|---|
| **v1.5.1-gs.3** (2026-09-11, tag on `2c8cacbb` = v1.5.1-gs.2 + the `GRPCRoute` method-match translation fix (pull request #6: a service-only match is a path prefix, `type: RegularExpression` is honoured); [run 34550503402](https://github.com/giantswarm/agentgateway-upstream/actions/runs/34550503402)) | v1.5.0 | `agentgateway:v1.5.1-gs.3` `sha256:98ea7da34357ab09c933d86c0421c77860131a05af664f74958c8d6b3043e929` · `controller:v1.5.1-gs.3` `sha256:0a4e033a7b6dcfad8dd2749c118206bc8090fe8bb9ab3e03b6876aee320deee0` (both linux/amd64 + linux/arm64, cosign-signed, Trivy clean) · charts `agentgateway:1.5.1-gs.3` `sha256:1312f4056d9f1185b396836f942bbc03e43e90ebc4dfa67431c95aa547ac194f`, `agentgateway-crds:1.5.1-gs.3` `sha256:1eb97f878f8eced81518ab2e4e03100c83613094f567ff759e4ab2237ab00943`, `agentgateway-standalone:1.5.1-gs.3` `sha256:57d48da620964180508326da17fb708c2e1a502218065f5bff62fcf4c96c2d11` (the `v1.5.1-gs.3` chart tags carry the same content under a second manifest) |
| **v1.5.1-gs.2** (2026-09-11, tag on `4d34ff55` = v1.5.0 + the grpc/x/crypto bump + agentgateway#3237's CONNECT-time actor egress authorization + the resuming-actor admission; [run 34544951234](https://github.com/giantswarm/agentgateway-upstream/actions/runs/34544951234)) | v1.5.0 | `agentgateway:v1.5.1-gs.2` `sha256:766f68bc1ec30c122615cd7d35d15ecada873f4a2623ff1b61fff67724145132` · `controller:v1.5.1-gs.2` `sha256:f0c7539b1b117ae4883f5de713f417d39b813c0f772c9d66c3890b1ba825ebfa` (both linux/amd64 + linux/arm64, cosign-signed, Trivy clean) · charts `agentgateway:1.5.1-gs.2` `sha256:af5b06c75db1c0b65fa79fd77a74ea5471c4f57c577fb39063b5e96761b7d1f2`, `agentgateway-crds:1.5.1-gs.2` `sha256:6bc86f889a512f06f12129580307704c66a59370081106561c5901b543cdb25f`, `agentgateway-standalone:1.5.1-gs.2` `sha256:c6868e6a2bd16e36dc3edb5769ad892728cda8378e79f6fbfa09755202c08c48` (the `v1.5.1-gs.2` chart tags carry the same content under a second manifest) |
| **v1.5.1-gs.1** (2026-09-10, tag on `5a5d5d8b` = v1.5.0 + the grpc/x/crypto bump; [run 34541814717](https://github.com/giantswarm/agentgateway-upstream/actions/runs/34541814717)) | v1.5.0 | `agentgateway:v1.5.1-gs.1` `sha256:e100bc9aea668ce0cc1c178057a34f1794f24a168d98d6da1144cfd4d4b0b7ee` · `controller:v1.5.1-gs.1` `sha256:78225d54d6e624582dc208eaa80fa89339da0346395bf67e38f40f5ae04b62e8` (both linux/amd64 + linux/arm64, cosign-signed, Trivy clean) · charts `agentgateway:1.5.1-gs.1` `sha256:60769399af5cfb479764b14054edc133a2c174d8762e726e9b905c3406230c3e`, `agentgateway-crds:1.5.1-gs.1` `sha256:d12a7166cdd8924cdcce632693896187050089bcda42cdc1ef704dd93f48c4da`, `agentgateway-standalone:1.5.1-gs.1` `sha256:08c33dcdb78521490a4075c4c9acd4fc911b0d093f0b34973d398bae90d0d79f` (the `v1.5.1-gs.1` chart tags carry the same content under a second manifest) |

**Scans.** Both own images are scanned with Trivy (HIGH and CRITICAL, fixable only) after the push and before the
charts that reference them are published. A fixable finding fails the publish: bump the dependency (upstream first)
or, when upstream has no fix, add a time-boxed entry to `.trivyignore` (`CVE-… exp:YYYY-MM-DD # reason, tracking
issue`) — an expired entry fails again and is re-triaged, not extended. Trivy reports four Istio advisories
(CVE-2019-14993, CVE-2021-39155, CVE-2021-39156, CVE-2022-23635) against `istio.io/istio`, which the controller imports at
a `v0.0.0-<date>` pseudo-version of a 2026 commit: Trivy cannot order a pseudo-version against the advisories' fixed
releases (Istio ≤ 1.13) and flags code that is years past them; they are time-boxed in `.trivyignore` until 2026-12-31
(re-triage: an `istio.io/istio` release tag in `go.mod`, or Trivy learning pseudo-versions). The controller's Go module
graph is covered by `govulncheck` on every push and pull request. Trivy does not see into a Rust binary that was not built with
`cargo auditable`; upstream's `deny.toml` (`cargo deny check advisories`) is the tool for the Rust dependency graph
and is not wired into CI yet — a known gap of the line, tracked in #37758.

**Builds are cold.** The Rust build runs inside the Dockerfile with BuildKit cache mounts that GitHub's cache does not
persist, so every publish rebuilds from scratch (the better part of an hour per platform). Good enough for a line that
rebuilds on re-pins and patches; a persisted cargo cache is an improvement, not a prerequisite.

## Consumers

| Consumer | Where the pin lives | Selects |
|---|---|---|
| [giantswarm/substrate](https://github.com/giantswarm/substrate) (the Substrate line) | `.github/workflows/publish.yaml` `AGENTGATEWAY_IMAGE`, stamped into the chart's `images.agentgateway` at package time | the line's release image for `atenet-router` and `atenet-egress` |
| [giantswarm/retagger](https://github.com/giantswarm/retagger) | `images/renamed-agentgateway.yaml` | mirrors the line's **release** tags (`vX.Y.Z-gs.N`, by `filter`) from this registry into `gsoci.azurecr.io/giantswarm/agentgateway` and `…/agentgateway-controller`, next to upstream's tags — the fleet pulls from gsoci only |
| [giantswarm/agentgateway](https://github.com/giantswarm/agentgateway) (the packaging chart) | `sync/patches/values/values.yaml` → `helm/agentgateway/values.yaml`: `controller.image.tag`, `proxy.image.tag` (Renovate capped to `-gs.` tags) | the gsoci mirror of a release; its chart stays upstream's, vendored from `cr.agentgateway.dev` until the line carries a chart patch (then `vendir.yml` points at this registry's charts) |
| [giantswarm/agent-platform](https://github.com/giantswarm/agent-platform) | the meta chart follows the packaging chart through `components.agentgateway.versionRange`; the connectivity chart pins the data-plane image in `agentgateway.proxy.image.tag` (rendered into `AgentgatewayParameters`) | the same release |

## Contributing

- **Upstream first.** Every behavioural change is a pull request to
  [agentgateway/agentgateway](https://github.com/agentgateway/agentgateway) with DCO sign-off (`git commit -s`);
  see upstream's `CONTRIBUTION.md`. The line carries the same change as a `git cherry-pick -x` of the upstream
  commit (or, before merge, of your pull-request branch) until an upstream release contains it, with a row in
  #37742. The upstream-facing branch and pull-request text are prepared here and listed in #37742 first; the team
  reviews that list and sends in batches (the epic's working rule).
- **Fork-only changes** (workflows, this file): a pull request from `fork/<topic>` against `giantswarm`; squash-merge.
  Cherry-picks of upstream commits are rebase-merged so they keep their patch-id and fall away at the re-pin.
- **Experiments**: your own personal fork. Branches here exist to become pull requests.
- **What CI runs on a pull request**: upstream's `Branch` workflow — proxy tests (Linux, with the Keycloak/JWKS
  validation dependencies), proxy lint (schema generation, clippy), UI lint and Playwright, controller tests and
  lint, controller e2e and both conformance suites on kind — plus `govulncheck`; `ci-ok` is required. `publish` runs
  only on the branch and on tags. Expect an hour: the Rust lanes build on 4-vCPU hosted runners with caches that
  only pushes to `giantswarm` roll forward.
- **Do not** dispatch upstream's `release.yml` or `nightly.yml` here (they are the mirror's files; they would push
  under upstream's names into this org's registry), and do not push tags other than `vX.Y.Z-gs.N` releases.
