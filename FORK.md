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
| Upstream tag | **v1.5.0** (2026-08-27; tag commit `fe673247`, "Substrate Refinements (#3174)") |
| Why this one | Both consumers run it. The packaging chart `giantswarm/agentgateway` vendors upstream's chart v1.5.0 and its images. The Substrate line (`giantswarm/substrate`, pinned to kagent-dev/substrate **v0.0.26**) deploys `ghcr.io/kagent-dev/substrate/agentgateway:c0f5597c7cb8` for atenet-router and atenet-egress — a build from upstream's Dockerfile dated 2026-08-30 without a revision label: a pre-merge build of agentgateway#3237 (the CONNECT-time actor authorization, merged 2026-09-01, in no release yet), so v1.5.0 plus that change is the substrate support the Substrate line's router and egress expect — the line carries it (see "Carried patches"). |
| When it moves | with the consumers, proven in agentlab first (`agentlab configure --defaults --chart-branch poc/kagent-main && agentlab up` and the proofs) — see "Re-pin" and "Convergence with the Substrate line". Not on a schedule. |
| Derived how | `git describe --tags --abbrev=0 --match 'v[0-9]*' --exclude '*-*' giantswarm` with upstream's tags fetched; the line's own tags carry a pre-release suffix and are excluded. The workflows compute it, nothing records it twice. |

### Convergence with the Substrate line

Upstream kagent-dev/substrate `main` moved its router image to `ghcr.io/agentgateway/agentgateway:v0.0.0-alpha.9f9744cf`
([kagent-dev/substrate#28](https://github.com/kagent-dev/substrate/pull/28), 2026-09-10) — a nightly of upstream
agentgateway commit `9f9744cf` ("substrate: Fix custom port (#3428)", 117 commits past v1.5.0). Between v1.5.0 and it
upstream changed the substrate protocol: #3237 (authorize actor egress at CONNECT time), #3289, #3333 (router metrics),
#3335, #3318 (egress policy), **#3409 (the new substrate ingress header)**, #3428. The Substrate line's next re-pin, onto
the first release containing kagent-dev/substrate#28, therefore needs an agentgateway ≥ `9f9744cf`: this line moves to
the first upstream release that contains those commits (v1.6.0 by upstream's cadence) or carries them until then. If at
that point the packaging chart cannot follow the same release, the platform's data-plane release stays the line and
the Substrate router goes back to a digest mirror of upstream's alpha until both converge — one branch serves both
consumers only while their pins agree.

## Carried patches

Everything on `giantswarm` that is not in the pin (`git log v1.5.0..giantswarm`):

| Patch | Purpose | Fork commit | Upstream |
|---|---|---|---|
| Authorize an actor's egress at CONNECT time: the egress dataplane reads the actor (atespace, name, UID, purpose `atunnel`) from the tunnel's client certificate and asks ate-api (`GetActor`) whether that UID is the actor's and the actor is placed on a worker before it opens the tunnel — upstream agentgateway#3237, `cherry-pick -x` of `8cbb254d`; `schema/config.{json,md}` regenerated against the pin (`make generate-schema`) | the Substrate line's `atenet-egress` runs this check in the request path (its egress config carries the `substrateEgress` policy and no `ext_proc`, so atenet's handler is not consulted); kagent-dev's `c0f5597c7cb8`, the image Substrate v0.0.26 pinned, was a pre-merge build of it. The line keeps that authorization rather than falling back to v1.5.0's, which derives the actor from the SPIFFE id and checks nothing else | the `substrate: authorize actor egress at CONNECT time (#3237)` commit of pull request #4 | merged in upstream `main` on 2026-09-01, not in v1.5.0. Falls away at the re-pin onto the first release containing it — the rebase will conflict on the regenerated `schema/` files, and the resolution is to drop this patch |
| Admit a `RESUMING` actor at CONNECT time next to `RUNNING` (SUSPENDED, PAUSED, CRASHED and DELETING stay refused; the denial names the state) | Substrate commits `RUNNING` only after the workload served readyz; a workload that fetches what it needs to become ready — kagent's Go ADK and Claude harnesses materialise git skills before readyz — was refused (`403 Forbidden: actor is not running`) and its ActorTemplate never got its golden snapshot ([#37742](https://github.com/giantswarm/giantswarm/issues/37742) row 8; acceptance test `agentlab skills-test`, [agentlab#137](https://github.com/giantswarm/agentlab/issues/137)). Counterpart of the Substrate line's [giantswarm/substrate#4](https://github.com/giantswarm/substrate/pull/4) (ateom arms the tunnel before the first container starts, atenet's `ext_proc` admits `RESUMING`) | the `substrate: admit a resuming actor's egress at CONNECT time` commit of pull request #4 — the same change in `egress.rs`, where the check lives on this base | to file: the upstream-shaped patch is branch [`upstream/substrate-egress-resuming`](https://github.com/giantswarm/agentgateway-upstream/tree/upstream/substrate-egress-resuming) here (`7771e400`, on the mirror `main`, where the check lives in `egress_actor_resolution.rs` since #3318); a team member opens the agentgateway pull request with DCO sign-off once #37742 has reviewed it |
| Bump `google.golang.org/grpc` to v1.83.2 and `golang.org/x/crypto` to v0.55.0 in the controller module (`go get` + `go mod tidy`; lifts `x/net` 0.58.0 and `x/text` 0.41.0 with them) | Trivy on the controller image, the publish gate: CRITICAL CVE-2026-56854 (`x/crypto/ssh`, fixed 0.55.0), HIGH CVE-2026-84304 and CVE-2026-84445 (grpc, fixed 1.83.1 / 1.83.2) | the `fix(deps)` commit of pull request #2 | upstream `main` has both since #3295 (grpc, 2026-09-02) and #3303 (all Go dependencies, 2026-09-03); not for upstream. Falls away at the re-pin onto the first release containing them — the rebase will conflict on `go.mod`/`go.sum`, and the resolution is to drop this patch |
| Fork infrastructure: this file, the README pointer, `CODEOWNERS`, `.github/workflows/publish.yaml`, `.github/workflows/sync-upstream.yaml`, `.trivyignore`; `pull_request.yml` on `giantswarm` with GitHub-hosted runners, Linux lanes only, `govulncheck` and `ci-ok`; `release.yml` without its tag trigger | the line's CI, publishing and sync | the `giantswarm` branch history | not for upstream |

Two patches change agentgateway's behaviour beyond upstream v1.5.0: the CONNECT-time actor authorization upstream has
merged since, and the admission of a resuming actor, written for upstream and leaving at the first release that carries it;
the dependency bumps are what upstream has since made itself. Giant Swarm specific configuration lives
elsewhere: the gateway's values, routes and policies in the [agent-platform](https://github.com/giantswarm/agent-platform)
meta and connectivity charts, the packaging in `giantswarm/agentgateway`, the Substrate router's agentgateway
configuration in the Substrate chart.

## Re-pin

The re-pin moves the line onto a new upstream release tag and replays the carried patches; a patch upstream has
merged falls away by itself (`git rebase` drops already-applied patches). It is the one sanctioned rewrite of
`giantswarm`.

1. Decide the tag with the consumers: the packaging chart's vendored chart version and the Substrate line's router
   protocol (see "Convergence") must both be served by it.
2. Run **Actions → sync-upstream → Run workflow** with `pin` = the tag (for example `v1.6.0`). The workflow mirrors
   `main`, rebases the carried patches onto the tag, builds the controller as a smoke, and force-pushes `giantswarm`.
   The push runs upstream's suite (`pull_request.yml`) and `publish` builds the dev build.
   - On a conflict it pushes `sync/<date>-<tag>` (the new tag + the patches that applied before the conflict) and
     opens a pull request that names the conflicting patch and the ones behind it. Finish it by hand: check the
     branch out, `git cherry-pick -x` the rest, resolve, test, `git push --force-with-lease origin HEAD:giantswarm`,
     close the pull request. **Do not merge it** — the line is a rebased branch; a merge would fold the old pin back in.
   - `dry_run: true` does everything except the pushes; the run summary shows the outcome.
3. Update this file (pin, carried patches) in a pull request, and the agentgateway rows of #37742.
4. Tag a release (`vX.Y.Z-gs.1`), move the consumers to it (see "Consumers"), prove it in agentlab.

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
git rebase --onto v1.6.0 v1.5.0          # new pin, old pin
go build ./... && make lint
git push --force-with-lease origin giantswarm
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
