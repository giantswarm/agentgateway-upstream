# The Giant Swarm line of agentgateway

This repository is Team Bumblebee's fork of [agentgateway/agentgateway](https://github.com/agentgateway/agentgateway).
The Giant Swarm Agent Platform runs agentgateway in two places — as the platform's gateway (the controller and the
data-plane proxy the [`giantswarm/agentgateway`](https://github.com/giantswarm/agentgateway) packaging chart
deploys) and inside Agent Substrate (`atenet-router` and `atenet-egress` of the
[`giantswarm/substrate`](https://github.com/giantswarm/substrate) line) — and this fork is where the agentgateway
they run is pinned, built, scanned and published. It answers one question from one place: **which agentgateway are
we running, and why does it differ from upstream?**

Tracking: the fork's own issues are filed in this repository's tracker; the cross-repo tracking is the line [giantswarm/giantswarm#37758](https://github.com/giantswarm/giantswarm/issues/37758),
upstream engagement [giantswarm/giantswarm#37742](https://github.com/giantswarm/giantswarm/issues/37742), the sibling
lines built the same way — kagent [giantswarm/giantswarm#37010](https://github.com/giantswarm/giantswarm/issues/37010)
(`giantswarm/kagent-upstream`) and Substrate [giantswarm/giantswarm#37757](https://github.com/giantswarm/giantswarm/issues/37757)
(`giantswarm/substrate`) — and the epic [giantswarm/giantswarm#37705](https://github.com/giantswarm/giantswarm/issues/37705).

## Branches

| Branch | What it is | Who moves it |
|---|---|---|
| `main` | A pure mirror of upstream `main`, fast-forwarded. Never edited, never the target of a pull request. | the sync workflow (as the HeraldBot App); the `protect-main` ruleset admits the App and repository admins (manual repair) and nobody else |
| `giantswarm` (default) | **The line**: the upstream release tag the platform runs ("the pin") + cherry-picked upstream fixes + this fork's own files. Every change of the fork's own files is a pull request against it. | pull requests (`ci-ok` required); a re-pin lands as the sync workflow's push of a candidate whose head passed `ci-ok` |
| `fork/<topic>` | pull-request branches against `giantswarm` | anyone in the team |
| `sync/<date>-<pin>` | re-pin candidates: the rebased line, checked by `ci-ok` on their push, landed by the sync workflow | the sync workflow; a human finishes one whose replay conflicted |

The line was bootstrapped on 2026-09-11 by pushing the tag commit of the pin to `giantswarm` by hand (the one push to
the branch that was not a pull request or a re-pin), then adding the fork's files through pull request #1.

## Pin

| | |
|---|---|
| Upstream commit | **`v1.6.0`** (tag commit `ea560864`, "Update model catalog (#3750)", 2026-10-02; 316 commits past v1.5.0). A release tag again: v1.6.0 is upstream's first stable release since v1.5.0 (2026-08-27) and the first that contains `8dba3989` (the egress credential injection the Substrate line's pin needs, see "Convergence"), `3528a428` (the previous pin), and the fixes that supersede two carried patches (#3723 `2353a2c1`, #3704 `f6ceca24`). It also contains agentgateway#3677 (`9e78d1da`, 2026-09-26): the egress authorizes an actor from the `spiffe://substrate-actor.local/ateom-for-actor/<atespace>/<actor>` certificate URI that agent-substrate/substrate#1809 (`8d6be5fb`) introduced and no longer reads the `ActorIdentity` extension, and credential providers receive `spiffe://substrate-actor.local/actor/<atespace>/<actor>`. |
| Why this one | The line returns to a release tag at the first stable release that contains `8dba3989`, as the previous pin's row promised, and v1.6.0 is that release. The packaging chart `giantswarm/agentgateway` (the platform's controller and data plane) follows this release; its vendored chart can move to upstream's v1.6.0 chart at the same time (see "Consumers"). **The Substrate line does not follow it yet**: `atenet-router` and `atenet-egress` of a release on this pin refuse every actor's CONNECT (the certificate carries the `ActorIdentity` extension and the old `spiffe://substrate-actor.local/atespace/<atespace>/actor/<actor>` URI, which #3677 no longer reads) until `giantswarm/substrate` carries agent-substrate/substrate#1809, which its pin (kagent-dev/substrate v0.2.0-beta5) does not. The Substrate chart keeps `2.2.2` until that move, which is the Substrate line's re-pin, proven in agentlab with this line's release (giantswarm/giantswarm#37742). |
| When it moves | with the consumers, proven in agentlab first — see "Re-pin". Not on a schedule. Previous pins: v1.5.0 (2026-09-10 → 2026-09-14, releases `v1.5.1-gs.1`–`gs.3`), `main` @ `c1d24607` (2026-09-14 → 2026-09-22, releases `v1.5.1-gs.4`, `v1.5.1-gs.5`, `2.0.0`), `main` @ `08f4563b` (2026-09-22 → 2026-09-24, releases `2.1.0`, `2.1.1`, `2.1.2`), `main` @ `3528a428` (2026-09-24 → 2026-10-05, releases `2.2.0`, `2.2.1`, `2.2.2`, `2.2.3-rc.1`); this pin's release is `2.3.0`. |
| Derived how | The pin itself is `git merge-base giantswarm main` (the mirror — after a re-pin onto an upstream commit newer than the last mirror run, `main` lags the pin until the next weekly run and the merge-base is the older commit; this row is authoritative). The upstream release it corresponds to is `git describe --tags --abbrev=0 --match 'v[0-9]*' --exclude '*-*' "$(git merge-base giantswarm main)"` with upstream's tags fetched — the nearest upstream release at or below the pin, v1.5.0 today. `git describe` runs at the merge-base, not at `giantswarm`, so the line's own stable tags (`v2.0.0`, …; see "Publishing") never count. The line's versions are decoupled from upstream's; the pin is documented here and stamped on every published index as the annotation `io.giantswarm.upstream.version`. |

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

Upstream kagent-dev/substrate then moved the router image to `ghcr.io/agentgateway/agentgateway:v0.0.0-alpha.8dba3989`
(in v0.2.0-beta4 and v0.2.0-beta5), a nightly of upstream commit `8dba3989` ("substrate: align credential injection
with provider contract (#3524)", 2026-09-17). Between `c1d24607` and it the substrate module changed twice: #3411
(egress credential injection — the egress gateway injects a destination's credential from a provider, so a Secret
value never enters a runtime) and #3524 (its provider contract). kagent-dev/kagent#2868 ("use Substrate beta4 gateway
credential injection") made kagent `main` depend on it. The Substrate line's re-pin onto v0.2.0-beta4 (2026-09-22)
therefore moved this line onto `main` ≥ `8dba3989` first; the Substrate chart pins this line's `2.1.0`, then `2.1.2`. The Substrate line's re-pin onto v0.2.0-beta5 (2026-09-24) needs nothing new from this line (beta5 runs the same `8dba3989` nightly) and keeps the chart at `2.1.2`.

## Carried patches

Everything on `giantswarm` that is not in the pin (`git log main..giantswarm`, the mirror `main` being the pin). Each carried change has a ledger file of its own in [`fork/`](fork/), so concurrent pull requests never edit the same lines; `ls fork/` is the index. A pull request that adds a carried change adds its file, one that drops it marks it `dropped` or deletes it (see "Re-pin").

A ledger file `fork/<slug>.md` is:

```markdown
# <Title: what the change does, as a sentence>

- **Status:** carried | dropped
- **Upstream:** the upstream pull request, issue or branch, or why the change is not for upstream
- **Our commit:** the commit on `giantswarm` and the pull request or issue it came from

## Change

What the change does: the files and symbols it touches.

## Reason

Why the fork carries it, and when it drops.
```

A value that is not known is written as unknown, never guessed.

Twelve carried patches sit on the pin, one commit and one ledger file each in `fork/`, beside the file for the fork infrastructure (the Kubernetes JWKS refetch is two commits, filed upstream as one). Nine change agentgateway's behaviour: the admission of a resuming
actor, the file resources read from disk on every reload together with the parse-time files made managed dependencies,
the resume refusal answered at once (#29, which leaves with Substrate's directive), the unreachable policy service
answered as unavailable (#35), a credential provider's failure answered by its class (#39), the JWKS refetch on an
unknown key id for local configuration (agentgateway#3037) and under Kubernetes, and the gRPC status of a trailers-only refusal in the access log. Checked against upstream `main` @ `85d1f087` on 2026-10-09, they
fall in four groups. Already upstream after the pin, so they drop at the re-pin onto a commit containing both: the parse-time files
(agentgateway#3863, `0a8476fe`) with the read on every reload, a workaround that leaves with it, and the unreachable policy service
(agentgateway#3793, `0c62b5a3`). To file: the admission of a resuming actor (after agent-substrate/substrate#2117), the credential
provider's failure class, widened to the policy and identity calls and to the local errors #3793 still answers as a denial, and the
gRPC status in the access log, with the request limit refusal. Waiting on others: the JWKS refetch for local configuration
(agentgateway#3037), and under Kubernetes an issue for the controller honouring `Cache-Control` first. Not for upstream as is: the
resume refusal answered at once, whose directive only Substrate 1.3 still sends and whose proper fix is Substrate's codes. The grpc pin at v1.83.2, the Go 1.27.2 / `x/net` v0.60.0 build and the `x/net` v0.61.0 requirement change versions only and leave when upstream requires fixed releases (see their files in `fork/`).

Dropped at the 2026-10-05 re-pin because the pin contains their fixes: the `GRPCRoute` method-match translation (`223b80e2`, pull request #6;
agentgateway#3723 `2353a2c1`, merged 2026-10-01, closes our agentgateway#3715 and makes a service-only match
a `PathPrefix` and a `RegularExpression` match one path regex; #37742 row 25) and the e2e warm step's absolute `-toolexec` path (`a2a16cea`,
pull request #21; agentgateway#3704 `f6ceca24`, merged 2026-09-30, resolves the helper from the repository root
the same way, so branch `upstream/ci-toolexec-absolute` is obsolete; #37742 row 55). Both conflicted at the rebase and were skipped.
The pin also brought agentgateway#3686 (`5bc6301f`, egress certificate rotation on long-lived tunnels and traced egress gRPC calls),
which sits next to #20/#23 without replacing them, and #3689 (`c4143926`, protocol selection from the Substrate policy), on which #35 and
#39 were re-seated.
Reverted on 2026-10-02 because upstream declined it: the named buffer error on an MCP upstream's response (#17,
`3afc3c82`, in 2.1.0 to 2.2.2-rc.2; declined in review on agentgateway#3714, closed). The old message already names the buffer
limit, and the platform sets its own size (`gateway.http.maxBufferSize`, giantswarm/agent-platform#630). Nothing was dropped at the 2026-09-24 re-pin: upstream merged none of the carried patches
between `08f4563b` and `3528a428`. Dropped at the 2026-09-22 re-pin because the pin contains them: the `cherry-pick -x` of upstream
`3d8dbe0a` ("Fix GIE dependency bump automation (#3252)") and `1f7ebbf8` ("gie: move to v1.6.2 off our fork (#3541)"),
carried between 2026-09-19 and 2026-09-22 because the suite's GIE conformance lane could not run without them (the
staging endpoint-picker tag it pulled had gone). Dropped at the 2026-09-14 re-pin because upstream had merged them: the `cherry-pick -x` of agentgateway#3237
(the CONNECT-time actor authorization, upstream `8cbb254d`) and the grpc/x/crypto dependency bumps (upstream #3295 and
#3303). Giant Swarm specific configuration lives elsewhere: the gateway's values, routes and policies in the
[agent-platform](https://github.com/giantswarm/agent-platform) meta and connectivity charts, the packaging in
`giantswarm/agentgateway`, the Substrate router's agentgateway configuration in the Substrate chart.

## Re-pin

The re-pin moves the line onto a new upstream commit — a release tag, or a `main` commit while upstream has no
release with what the consumers need — and replays the carried patches; a patch upstream has merged falls away by
itself (`git rebase` drops already-applied patches), and its file in `fork/` is marked `dropped` or deleted. It is the one sanctioned rewrite of `giantswarm`.

1. Decide the pin with the consumers: the packaging chart's vendored chart version and the Substrate line's router
   protocol (see "Convergence") must both be served by it. The three lines move in one order — this line first, then
   the Substrate line (its chart pins this line's release), then the kagent line (its `go.mod` pins Substrate).
2. Run **Actions → sync-upstream → Run workflow** with `pin` = the tag or commit (for example `v1.6.0`). The workflow
   mirrors `main`, rebases the carried patches onto the pin, builds the controller as a smoke, pushes the result as the
   candidate `sync/<date>-<pin>` and opens a pull request against `giantswarm` that records the line head it was
   replayed from. The candidate's push runs `ci-ok` (`pull_request.yml` runs on `sync/**` pushes — the pull request
   itself runs nothing, a rebased branch has no merge commit). A new candidate supersedes the older `sync/*` branches:
   their pull requests are closed and the branches deleted.
   - On a conflict the candidate holds the new pin + the patches that applied before the conflict, and the pull
     request names the conflicting patch and the ones behind it. Finish it by hand: check the branch out,
     `git cherry-pick -x` the rest, resolve, test, push the branch, which runs `ci-ok` on its head.
   - `dry_run: true` does everything except the pushes; the run summary shows the outcome.
3. Once `ci-ok` is green on the candidate's head, run **Actions → sync-upstream → Run workflow** with `land` =
   `sync/<date>-<pin>`. It lands the candidate only when `ci-ok` passed on its head and `giantswarm` is still at the
   recorded head: it pushes exactly that commit to `giantswarm` with the lease on that head, closes the pull request
   and deletes the branch. The push publishes the dev build. **Do not merge the pull request** — the line is a
   rebased branch; a merge would fold the old pin back in. The `devctl: line gate` ruleset refuses every push to
   `giantswarm` whose head has not passed `ci-ok`, for every actor, so nobody updates the line by hand.
4. Update this file (the pin) and the ledger files in `fork/` (a dropped patch is marked `dropped` or deleted) and the upstream-pin annotation in `.circleci/config.yml`
   (`index:io.giantswarm.upstream.version=<pin>`, stamped on every published index) in a pull request, and the
   agentgateway rows of #37742.
5. Tag a release (`vX.Y.Z` — a re-pin is at least a minor bump, see "Publishing"), move the consumers to it (see
   "Consumers"), prove it in agentlab — for a move the
   Substrate line depends on, the release is cut once `ci-ok` is green on the exact head (the Substrate chart's
   publish resolves the release image by digest) and its pipeline is green (the images are pushed before the scan,
   so a red release exists in the registry and is not consumed), and the agentlab proof runs on the three lines'
   builds together before the Substrate and kagent releases follow.

The 2026-09-14 re-pin (v1.5.0 → `main` @ `c1d24607`) by hand, in a scratch worktree: `git rebase --onto upstream/main
v1.5.0` dropped the deps bump and the #3237 cherry-pick (`git rebase --skip` on their `go.mod`/`schema` conflicts),
the resuming admission conflicted in `egress.rs` and was replaced by `git cherry-pick 7771e400` (the prepared
main-shaped commit), the `GRPCRoute` fix replayed clean; `go build ./...` and `cargo fmt --check` locally, the Rust
lanes in CI on the `sync/20260914-main-c1d24607` push; the force-push by a repository admin (a bypass actor of the
`giantswarm` ruleset). `schema/config.{json,md}` needed no regeneration (the remaining Rust patch touches no config type).

The 2026-09-22 re-pin (`main` @ `c1d24607` → `main` @ `08f4563b`) by hand as well: the workflow's dry run stopped on
the FORK.md release-table commit because it rebases from `git describe`'s v1.5.0 rather than from the merge-base; in a
scratch worktree `git rebase --onto upstream/main c1d24607` conflicted only on the first GIE cherry-pick (`go.mod`,
`go.sum`, the OSS compliance list — skipped, upstream has it) and dropped the second by patch id; the resuming
admission and the `GRPCRoute` fix replayed clean. `go build ./...`, the translator tests and `cargo fmt --check`
locally; the suite in CI on the `sync/20260922-08f4563b` push; the force-push by a repository admin.

The 2026-09-24 re-pin (`main` @ `08f4563b` → `main` @ `3528a428`) by hand as well, in a scratch worktree: `git rebase
--onto upstream/main 08f4563b` replayed all 27 carried commits and conflicted once, in the parse-time file dependencies
patch (#23) — upstream #3639 replaced the YAML library (`serdes::yamlviajson::from_str` → `serdes::yaml::from_str`) on the
line the patch wraps in `record_files_only_reads`; resolved to the patch on the new parser. `cargo fmt --check`, `go build
./...` and the controller's Go tests locally; the suite in CI on the `sync/20260924-3528a428` push, where `govulncheck`
flagged upstream's grpc v1.84.0 (GO-2026-6443) and the line gained its grpc pin (above); the force-push by a repository admin — who first lifts the admin bypass of the
repository manager's `devctl: default branch` ruleset (present since 2026-09-24; it admits admins for pull requests only
and refuses a force-push otherwise) to `always` and restores it right after.

The 2026-10-05 re-pin (`main` @ `3528a428` → `v1.6.0`) by hand as well, in a scratch worktree: `git rebase --onto v1.6.0 3528a428`
replayed 42 carried commits with four conflicts. The `GRPCRoute` translation and the `-toolexec` path conflicted because upstream
had fixed both (#3723, #3704) and were skipped. The resuming admission conflicted in `egress_actor_resolution.rs`, rewritten by
#3677: the patch's UID comparison left with the certificate extension, the placement check (`placed_on_worker`) replaced upstream's
`Running`-only check on `GetActor`'s state. #35 and #39 conflicted in `egress.rs`, restructured by #3689 (`fetch_policy`, the
`FetchSecret` span): re-seated on the new shape, the shared `policy_service_unavailable` kept in `mod.rs`. The grpc pin replayed
clean. `cargo fmt --check`, `go build ./...` and the translator tests locally; the suite in CI on the `sync/20261005-v1.6.0` push;
the force-push by a repository admin.

The weekly run (Mondays 05:41 UTC) does not re-pin: it mirrors `main` and **probes** whether the carried patches
still rebase onto upstream `main`, naming the first patch that would conflict in the run summary, so the next
re-pin is never a surprise.

**Identity of the automation.** The workflow pushes as the org's **HeraldBot GitHub App** — a token minted per run
from the org secrets `HERALD_CLIENT_ID` / `HERALD_APP_KEY` (`actions/create-github-app-token`); the App is installed
on every org repository with contents and workflows write access and is a bypass actor (`Integration`) of the
rulesets on `main` and `giantswarm`; on `giantswarm` the line gate still requires `ci-ok` on whatever it pushes. Why an App and not the org's machine-account token: GitHub refuses a push from a personal access token
that creates or changes a file under `.github/workflows/` unless the token carries the `workflow` scope, and upstream
`main` — hence every mirror and every re-pin — carries upstream's workflow files. A push with the workflow's own
`GITHUB_TOKEN` would not do either: it triggers no other workflow, and the push to `giantswarm` is what publishes the
dev build. The mirror push does start the workflows upstream's own files define on `main` (they ask for Blacksmith
runners the org does not have); the sync cancels them.

Building a candidate by hand (a workstation, upstream as a remote) ends at the candidate's push; the land step
above lands it:

```sh
git fetch upstream main 'refs/tags/v*:refs/tags/v*'
git checkout -b sync/$(date -u +%Y%m%d)-v1.6.0 origin/giantswarm
git rebase --onto v1.6.0 "$(git merge-base origin/giantswarm upstream/main)"   # new pin, old pin (a tag or a main commit)
go build ./... && make lint
git push origin HEAD                                                            # ci-ok on the candidate
```

Open its pull request against `giantswarm` with the last body line `Line head: <the full sha of giantswarm it was
replayed from>`, then run the land step.

## Publishing

`.circleci/config.yml` publishes the line's two images to the org's registries — `gsoci.azurecr.io` first, the China
mirror for releases — on every push to `giantswarm` (a dev build) and on every release tag (`vX.Y.Z`, or `vX.Y.Z-rc.N`
for a release candidate); the tags themselves are cut by the org's `zz_generated.auto_release.yaml` workflow (an `-rc.N` tag and its GitHub Release on every push to `giantswarm`, computed by git-cliff from the conventional commits since the last reachable tag; a stable tag through its `workflow_dispatch`) or by hand for a re-pin, whose rewritten history hides the last rc tag from the workflow (on 2026-10-05 it failed with `Release.tag_name already exists` until `v2.3.0` existed); no image is published from GitHub Actions or by hand, and nothing pushes to ghcr.io (the
line published there from a fork workflow until 2026-09-19; that publication was a mistake and its retagger copies
into gsoci are retired, [giantswarm/giantswarm#37874](https://github.com/giantswarm/giantswarm/issues/37874)). The
pipeline is the architect orb's stock jobs (`giantswarm/architect@10.10.0`; 10.6.0 was the first release with `build-args`) and
nothing else: `build-image` builds one platform natively on a machine of that architecture and pushes it by digest,
`push-to-registries` computes the version (`gitsemver`), builds or merges, tags, signs and attests (SLSA provenance,
SPDX SBOM) with the org's CircleCI identity, `sync-china-registry` mirrors a release; the fork adds a Trivy scan of
each image after its push. Both images are built for linux/amd64 and linux/arm64.

| Artifact | Name |
|---|---|
| Data-plane proxy | `gsoci.azurecr.io/giantswarm/agentgateway-upstream/agentgateway:<version>` — upstream's root `Dockerfile` unchanged (Rust 1.98 on `chainguard/glibc-dynamic`, the UI embedded); each platform built natively by `build-image` (`xlarge` / `arm.xlarge`) and joined into one index by `push-to-registries` (`merge-digests`); the build arguments `VERSION=${DOCKER_IMAGE_TAG}` and `GIT_REVISION=${CIRCLE_SHA1}` come from the job (`build-args`) |
| Controller | `gsoci.azurecr.io/giantswarm/agentgateway-upstream/controller:<version>` — `.circleci/Dockerfile.controller`: a multi-stage build that cross-compiles the Go binary on the build host (`CGO_ENABLED=0`, upstream's ldflags, `VERSION=${DOCKER_IMAGE_TAG}`) and copies it onto `chainguard/static` (user 10101, upstream's entrypoint), as one multi-platform `push-to-registries` build — nothing runs emulated |

**The names are the line's own**, nested under the repository's name like the kagent (`giantswarm/kagent-upstream/…`)
and Substrate (`giantswarm/substrate/…`) lines. The flattened repositories `gsoci.azurecr.io/giantswarm/agentgateway`
and `…/agentgateway-controller` receive nothing from here any more: they hold retagger's copies of **upstream's**
releases (`cr.agentgateway.dev`, bare `vX.Y.Z` tags) and the line's coupled releases up to `v1.5.1-gs.5`, which were
published there. Consumers move to the new names in their own pull requests (see "Consumers").

Not published from here: the charts (upstream's, vendored by the packaging chart from `cr.agentgateway.dev`), the
`agentgateway` and `agctl` binaries, the Windows image, the s390x image (use upstream's release for those), and the
OpenVEX attestation upstream attaches to the controller (the orb attaches provenance and SBOM instead).

**Versions.** The line's versions are its own, decoupled from upstream's, as the RFCs
[Semantic Versioning of Upstream Software](https://github.com/giantswarm/rfc/tree/main/semantic-versioning-of-upstream-software)
and [Semver Based Automatic Upgrades](https://github.com/giantswarm/rfc/tree/main/semver-based-automatic-upgrades)
("Tagging schema") lay down. The upstream version is the pin (see "Pin"), stamped on every published index as the
annotation `io.giantswarm.upstream.version` (`v1.6.0` today; the value lives in `.circleci/config.yml` and
moves with every re-pin) — `oras manifest fetch <image> | jq .annotations` reads it. Image tags are bare semver: the
git tag carries the `v`, the image tag does not (the orb's convention, and the sibling lines').

- **Release**, on a git tag `vX.Y.Z` (or `vX.Y.Z-rc.N` for a release candidate): image tag `X.Y.Z` (`X.Y.Z-rc.N`).
  The first decoupled release is **`2.0.0`**, the next major above the last coupled release (`v1.5.1-gs.5`), so a
  semver range that followed the line keeps ordering correctly and the break with the coupled scheme is visible. A
  re-pin is at least a minor bump, a carried patch or a pipeline change a patch bump, a change consumers have to
  follow a major. The fleet consumes releases only (see "Consumers"); a tag of any other shape runs no pipeline.
- **Dev build**, on every push to `giantswarm`: gitsemver's dev version of the commit — the next patch above the
  nearest stable tag reachable from it (with none reachable, as before `v2.0.0`, it starts from `0.0.0`; the line's
  `-gs.N` tags are pre-releases and do not count), then the dev pre-release part naming the branch, the committer time
  and the commit, in the schema of the gitsemver in the orb's architect image. A Flux consumer of the channel uses a
  `semverFilter` that pins the width of every field (the RFC's "How to select dev builds"). Nothing in the fleet
  consumes dev builds.
- **Any other branch** runs the same builds with `push: false` (the `validate` workflow): the Dockerfiles and the
  build arguments are proven on the pull request, nothing is pushed. A dev build of a change is its merge to
  `giantswarm`; there is no publish of another branch.

**Digests.** Every push job prints the digest of each pushed index; consumers pin by tag and verify by digest
(`crane digest`, and `cosign verify --certificate-oidc-issuer https://oidc.circleci.com --certificate-identity-regexp
'^https://circleci\\.com/api/v2/projects/[a-f0-9-]+/pipeline-definitions/[a-f0-9-]+$'` for the signature). Release digests
are recorded here (releases up to v1.5.1-gs.4 were published to `ghcr.io/giantswarm/agentgateway-upstream` by the fork
workflow and copied to the flattened gsoci names by retagger, v1.5.1-gs.5 to the flattened names from CircleCI; their
ghcr originals are not maintained):

| Release | Pin | Images and charts |
|---|---|---|
| **2.3.3** (2026-10-09, tag `v2.3.3` on `0c2c2ea0` = 2.3.2 + a remote JWKS refetched on an unknown key id before a token is refused, also when pushed under Kubernetes, and a held JWKS that has the asked kid answered without a refetch (#52, ledger #46) + a refusal answered trailers-only logging its gRPC status + Go 1.27.2 and `golang.org/x/net` v0.60.0 (#51); a patch of the line) | `v1.6.0` | `agentgateway-upstream/agentgateway:2.3.3` `sha256:8d27583d0dd62afbb3c77d76861697b62c7ea70adc14f22e29bfaf403a6cb5f7` (linux/amd64 `sha256:35a66c262316959f1855b624aef77492b00d419a6e2db8f6fe7a5eeaf739338e`, linux/arm64 `sha256:f0020fd6341c447d57d8a0f734e609c9b8610feef5870e6569ca104865c71151`) · `agentgateway-upstream/controller:2.3.3` `sha256:973a859a48a28a4e99eebcccbd5e2263c012baa2a0fdc4c4e618ace671f52784` (linux/amd64 `sha256:b2794930264a2d67796ca044d7bcae464b5180ea2517377c8b5e890a2387853c`, linux/arm64 `sha256:0fa815658c940963935859a1418bbcee42cdceba72f32422c25f15440c16334d`) |
| **2.3.2** (2026-10-08, tag `v2.3.2` on `3e7ce01b` = 2.3.1 − the gRPC caller's status from the resume (#44, reverted by #47): no caller reads it, and a parking budget spent retrying a status answered 429 instead of 504; the code of 2.3.0 again, the align-files update (#42) aside; a patch of the line) | `v1.6.0` | `agentgateway-upstream/agentgateway:2.3.2` `sha256:d43c7a3230b9c262486047e2c1101713ddb9bf1b7a5e83a399311c4a91f5c717` (linux/amd64 `sha256:71c6026e386ef50635eec4d1bd675a0fd75bcd0acc713b2e10eda715da0140a1`, linux/arm64 `sha256:4eef4329fa94048539246ffa8d6b76fa4617aba529ba780606376090db1284da`) · `agentgateway-upstream/controller:2.3.2` `sha256:27e19c05aa920cc1b8f705009d369143e038989584d036de5368423a5e9354ee` (linux/amd64 `sha256:9c1c87680263816adbaf2531322eff6d6572daa4eecade9f31ed4a924a7ce1d3`, linux/arm64 `sha256:b0d069110b52bd4cacb6470ad03dee4925507b4f396ad70650c81392206779fe`) |
| **2.3.1** (2026-10-08, tag `v2.3.1` on `680d6210` = 2.3.0 + a gRPC caller of the Substrate ingress answered with the resume's status (#44) + the align-files update (#42); **superseded by 2.3.2, do not pin**: its parking budget answers 429 where every other release answers 504) | `v1.6.0` | `agentgateway-upstream/agentgateway:2.3.1` `sha256:15805392e84a96eea819f35c9fe6f39cbdd0df49e0e811f38549d1cd3b798e70` (linux/amd64 `sha256:e816e8f22f5f19e15ee496fea20a84dacffcd0e2608c2aef0914def9100791b0`, linux/arm64 `sha256:04c070ec9d85d5b5e92788d2818e3efef99f2a2ace6dfa87ae9e3295cbe3545e`) · `agentgateway-upstream/controller:2.3.1` `sha256:ce711c41b57f80b6deeeeee1c0558a080a0f8cf451bcb7828b8eaaa602128cac` (linux/amd64 `sha256:7d7bce14bfb52f894a998c1cbd9f21a404702329fa12521660909d12e9126d9e`, linux/arm64 `sha256:b0735ecbe470a2ddc471f201a5efe5579f9cfdaff3dcebb6ab3789b4f7c48012`) |
| **2.3.0** (2026-10-05, tag `v2.3.0` on `97b142d6` = the 2026-10-05 re-pin onto upstream `v1.6.0` with the seven carried patches replayed (two dropped as fixed upstream, see "Carried patches") + the ledger; a minor of the line for the re-pin; CircleCI `publish` workflow of the tag green: both native data-plane builds, both pushes, both Trivy scans, both China syncs; **not for the Substrate line** until it carries agent-substrate/substrate#1809, see "Pin") | `v1.6.0` | `agentgateway-upstream/agentgateway:2.3.0` `sha256:b40b5f0fb888cee77347120e8bb40213a30964f6bcf123b3d63506d15d9f2755` (linux/amd64 `sha256:73d1db5117f3946644a47558dcd12df4152a0ee487cca401d7ccde8ab69b85cd`, linux/arm64 `sha256:46f975ae75f6e1cb3ac73322f126f0d278a545ab8e9a165d58bc203d39079085`) · `agentgateway-upstream/controller:2.3.0` `sha256:00c28885fc4c7efd3ce7bcd7e0a4daceac78a94c77990f295f695137aa641f1b` (linux/amd64 `sha256:9db4a608e17493d4741c37e69b0298b34413ff36954bec42648ced8d76817d96`, linux/arm64 `sha256:41b3369114ddfac14affc83e2b293bacd42c44d01b5ea29bc134571adc188708`) |
| **2.2.3-rc.1** (2026-10-02, tag `v2.2.3-rc.1` on `b84bc700` = 2.2.2 + a credential provider's failure answered by its class (#39) + the ledger (#38, #40); a release candidate, images in gsoci) | `main` @ `3528a428` | `agentgateway-upstream/agentgateway:2.2.3-rc.1` `sha256:657dbe614c8853ec885af009e6f47f485fe2ea954c6a2030fb130ee5ae272e06` (linux/amd64 `sha256:a6816e0f8c23fa289aae9eff23405e52e1db3e9178fe13c7eb305a972c4da45d`, linux/arm64 `sha256:b88ecb586008995c9d52e771368c3408d7e59a8d901b1d436b97ad9c135ff6f4`) · `agentgateway-upstream/controller:2.2.3-rc.1` `sha256:5215400db79704a5a65a16f69aed38c1313505c77492a2826963738ba530c672` (linux/amd64 `sha256:fa78966c95effdd6bacdfcb7c59f4b54277dfe5ff734c9e66e7b2f94ddea42c6`, linux/arm64 `sha256:a7a828f0480a70dfcea543d86cf5e808b0a37fd5d56a39e9287bc368ae7fb809`) |
| **2.2.2** (2026-10-02, tag `v2.2.2` on `149d9b40` = 2.2.1 + a Substrate policy call that never reached its service answered 503 instead of 403 (#35, rebase) − the named buffer error on an MCP upstream's response (#17, reverted by #36) + the align-files update and the ledger (#31, #33, #34); a patch of the line; the Substrate chart's pin, see "Consumers") | `main` @ `3528a428` | `agentgateway-upstream/agentgateway:2.2.2` `sha256:3b3b7b619bad285dbfab7fc0a51f0c9ff88a0de91bc4e9d5a340902f9b379c4a` (linux/amd64 `sha256:7a1f5faf4af1990024c4bfbf60ac32411196c347a7549365eefc482c99a70d8f`, linux/arm64 `sha256:e1ff2a9d65181c5b77cbebe5113fbf063603e3dfad88432a7d48a5b8fb11c757`) · `agentgateway-upstream/controller:2.2.2` `sha256:2518695b4ead3a8971303cb4925d8c97d1d03df475324f9add451c8ff136fe94` (linux/amd64 `sha256:6b8d9713920aedfe680f2b8022af1f6b0c3adf17ff39b19dfbf6e6328e0fd467`, linux/arm64 `sha256:c24ddbdbdc66b48f2e525a40f0e64fe4ec3281180bac33bfe59e0aadabad4ad7`) |
| **2.2.1** (2026-09-25, tag `v2.2.1` on `13325b82` = 2.2.0 + a resume refusal the Substrate control plane marks not resumable answered at once with its own status (#29, rebase); a patch of the line for a carried fix; CircleCI `publish` workflow of the tag green, `devctl release wait` exit 0; proven in agentlab as atenet-router and atenet-egress in the giantswarm/substrate#79 upgrade proof) | `main` @ `3528a428` | `agentgateway-upstream/agentgateway:2.2.1` `sha256:b3decd06d41ae4b9124210bfee2fb8529d704333827dfd8402c5e7eb1544d65c` (linux/amd64 `sha256:9a33e808e1813bb06d20db47121af985b0d600cd2f0357573c27827e326a758e`, linux/arm64 `sha256:3096db7f77406f8285534a1b2a4fb8abea90f6eba9ef837c7050a4874374ec92`) · `agentgateway-upstream/controller:2.2.1` `sha256:cfa5fedd5e3f6e1ca36507ee354238e63de368e7b0bdadb8ea146aeb17cc2157` (linux/amd64 `sha256:c7e9f07b216d55432b5c45adfa10925821a739a89dfaa5f5dc7b282a20a4f5ce`, linux/arm64 `sha256:59e2100a06da1db8c44994f2b12718aa755fe933e86174a44a6aa63a89c90e0a`) — both under `gsoci.azurecr.io/giantswarm/`, published like 2.2.0; no charts. Consumers: the Substrate line's `images.agentgateway` (giantswarm/substrate#81) |
| **2.2.0** (2026-09-24, tag `v2.2.0` on `71f1adf6` = the 2026-09-24 re-pin onto `main` @ `3528a428` with the six carried patches replayed + the grpc pin at v1.83.2 (GO-2026-6443) + the ledger (#27); a minor of the line for the re-pin; CircleCI `publish` workflow of the tag green, `devctl release wait` exit 0; proven in agentlab on the re-pin's dev build with the Substrate and kagent lines' dev builds: platform-test, a2a-test, skills-test public and private, agents-test) | `main` @ `3528a428` | `agentgateway-upstream/agentgateway:2.2.0` `sha256:370827be39bb2d8040af02cad69e0163e6ed0382c5565839dff4eadcbeea4b56` (linux/amd64 `sha256:0df237aca4d3cb74c9bad490585ee5ba6cd938b5b897f17ab5b7cbdb13a5c21a`, linux/arm64 `sha256:389d50723633ab8489cc9d84c77209e6429f560cda6bcb55e384335e8e596172`) · `agentgateway-upstream/controller:2.2.0` `sha256:5502d4f18c63a0b9cde577440ae7248532287d3b408ddeeca6f069d67f2b8c0d` (linux/amd64 `sha256:f7bfaacb8beea5b18ebb2009d8e4994cc9347c4be59cf5e8a37d29ed0893d4d3`, linux/arm64 `sha256:caa65ca59cf3869491786fe9f20e23e22d3c8e6e6ce981a765c3edec4bd84639`) — both under `gsoci.azurecr.io/giantswarm/`, published like 2.1.2; no charts. Consumers: the Substrate line's egress and router gateways from its `1.1.0` (giantswarm/substrate#74); the packaging chart and the platform's controller and data planes follow with their own bumps |
| **2.1.2** (2026-09-23, tag `v2.1.2` on `39023841` = 2.1.1 + the files a config parse reads made managed dependencies and the watch refresh re-armed (#23, rebase) + the e2e warm step's absolute `-toolexec` path (#21) + the ledger (#19); CircleCI `publish` workflow of the tag green, `devctl release wait` exit 0) | `main` @ `08f4563b` | `agentgateway-upstream/agentgateway:2.1.2` `sha256:a29ec8796363013f633bc04357392248619907c7a14bd5c481ee56f0b8acbd6a` (linux/amd64 `sha256:36d31954f5125dc1ce526e842c6659d0fedc1812249d046774959bcc27688cb2`, linux/arm64 `sha256:884899fbed473f0817e2dd9d3f1439e0a7470d837e4dbef34ae1d0ef779b7d21`) · `agentgateway-upstream/controller:2.1.2` `sha256:30116ff995b4e66f710173772fe74e56fb97c3358cf9e398c5c0a99cded98191` (linux/amd64 `sha256:1a5be548cf4e3433776590ffea9ee71e75e529ee6a2005a13e30b8453b8e5569`, linux/arm64 `sha256:74a6ec222e477f4f6e077590b7f0d876e9e62b01aa92dce221070a1872853894`) — both under `gsoci.azurecr.io/giantswarm/`, published like 2.1.1; no charts. Consumers: the Substrate line's egress and router gateways (release-1.0 v1.0.3, giantswarm/giantswarm#37915), the packaging chart (giantswarm/agentgateway#66), the platform's controller and data planes with agent-platform#648 |
| **2.1.1** (2026-09-23, tag `v2.1.1` on `db201f5e` = 2.1.0 + the file resources read from disk on every managed fetch and re-read every 60 s (#20, squash) + the align-files update (#18); CircleCI `publish` workflow of the tag, every job green: both native data-plane builds, both pushes, both China syncs, both Trivy scans) | `main` @ `08f4563b` | `agentgateway-upstream/agentgateway:2.1.1` `sha256:825c2f9e7156c59f333c840fc06c80f8cf3056a6f08e7646d58c3727871689e4` (linux/amd64 `sha256:69140cb9a724d912dea8de7341e3a5ece556868d6b11204b5bedc054a9edc239`, linux/arm64 `sha256:5415e673767ffdbb1bc21a4698f3e8e6625ed3d5b30b272ea4e68d2454049560`) · `agentgateway-upstream/controller:2.1.1` `sha256:adebf8c99ef67bedd687ab51ddfc25f54000d832f9ed7237bf222fb787cb360c` (linux/amd64 `sha256:7078d64656e605d48c7842c5ac542701191f3ef59ea8640b9dd88f99d879fffe`, linux/arm64 `sha256:70e7e53c1a54303430b9474c09620bd8d91442796ea7ef5a84d9d888cea06129`) — both under `gsoci.azurecr.io/giantswarm/`, cosign-signed with the org's CircleCI identity, SPDX SBOM and provenance attached, index annotation `io.giantswarm.upstream.version=main@08f4563b`, Trivy clean, mirrored to the China registry; no charts (see "Publishing"). Consumers: the Substrate line's egress and router gateways first (giantswarm/giantswarm#37915: the rotated podidentity certificate), through the Substrate release's `images.agentgateway` and the meta chart's `substrate.images.agentgateway`; the packaging chart (v2.6.0) and the platform's controller and data planes (agent-platform 4.65.0) followed the same evening |
| **2.1.0** (2026-09-23, tag `v2.1.0` on `7d08a380` = upstream `main` `08f4563b` (the 2026-09-22 re-pin) + the named buffer error on an MCP upstream's response (#17, squash) + the fork's files; CircleCI `publish` workflow of the tag, every job green: both native data-plane builds, both pushes, both China syncs, both Trivy scans) | `main` @ `08f4563b` | `agentgateway-upstream/agentgateway:2.1.0` `sha256:a1f3c45b1ffb21f320a08a82dfeb65a2db6b5b833c4417ee3a9ad3a05e41f45d` (linux/amd64 `sha256:af2434ca210398b1e503d7481483daeeabc0923a0bf33a56bb5e6bb54f29314e`, linux/arm64 `sha256:8e062a806a5e4adb5c3dc62cd979f74fa31646bf8afda52ad59115612d12570b`) · `agentgateway-upstream/controller:2.1.0` `sha256:2cd1c573ec9a151908d635b701fcfe334c0b8d8812eb1dc03072ae71d84a8c5c` (linux/amd64 `sha256:f20b29a467b39fb653152afd00a34c3c06fa24f42d5d5549c013cde78d88b10f`, linux/arm64 `sha256:5f76dfddf65032734a7b4e839a6e930288384ef9405a0751f2ba3ee00a9d9e99`) — both under `gsoci.azurecr.io/giantswarm/`, cosign-signed with the org's CircleCI identity, SPDX SBOM and provenance attached, index annotation `io.giantswarm.upstream.version=main@08f4563b`, Trivy clean, mirrored to the China registry; no charts (see "Publishing"). Consumers: the packaging chart (giantswarm/agentgateway#62) and the platform's controller and data planes (giantswarm/agent-platform#644); Substrate's egress gateway stays on 2.0.0 until the Substrate line's release names 2.1.0 |
| **2.0.0** (2026-09-19, tag `v2.0.0` on `8d92d51d` = upstream `main` `c1d24607` + the resuming-actor admission + the `GRPCRoute` method-match translation + the GIE v1.6.2 cherry-picks + the fork's files; CircleCI pipeline 15 — **the first decoupled release, through the orb's stock jobs, under the line's own names**) | `main` @ `c1d24607` | `agentgateway-upstream/agentgateway:2.0.0` `sha256:63deaa67438dc7bd9ec5c1cd1521489c783b3345a303f29c70d742559b665339` (linux/amd64 `sha256:b15a093c7e789b6db1e4848c77b0d63c71b612c476a96e565f78e6a4c52a0792`, linux/arm64 `sha256:28fc9b451adef574a625ff32aa5a05982042218fda0051738f8c57ea9657e040`) · `agentgateway-upstream/controller:2.0.0` `sha256:19b896b1467d796766470a71e130f0f38f3f421a8170f31bf2c1968050841626` (linux/amd64 `sha256:755b5116603e6a33e1de3f7a65cf462d7f3d20a6a1efcfcce8f6f98f151d940d`, linux/arm64 `sha256:e455cfc25e88421808d8cedec3b5e24d423e0985d38e6bfbe38d8d0d5611ff33`) — both under `gsoci.azurecr.io/giantswarm/`, cosign-signed with the org's CircleCI identity, SPDX SBOM and provenance attached, index annotation `io.giantswarm.upstream.version=main@c1d24607`, Trivy clean, mirrored to the China registry; no charts (see "Publishing"). The first stock dev build of the consumed branch (pipeline 14, the merge commit) landed the same way |
| **v1.5.1-gs.5** (2026-09-19, tag on `f72ac66c` = upstream `main` `c1d24607` + the resuming-actor admission + the `GRPCRoute` method-match translation + the GIE v1.6.2 cherry-picks (`af8f7fbc`, `501646d0`) + the fork's files; CircleCI pipeline 8 — **the first release published from CircleCI to gsoci**, nothing on ghcr) | `main` @ `c1d24607` | `agentgateway:v1.5.1-gs.5` `sha256:d3fca0fa05d19e0d7e507fa5f9f9ea28a1c0570c15fb302cad3d50d69a1cdcea` (linux/amd64 `sha256:64faaf41791a25885b7e65f46d8ab5adfa401279c9c1a5ba3a83f5890a98927b`, linux/arm64 `sha256:d181f4d42d46d2f2b631f60bd43c096f434b4d86e7b015486cb35068dde60d24`) · `agentgateway-controller:v1.5.1-gs.5` `sha256:d5bf7ac4a9cadb1faf262cc54983d86bbbb3502f9c22150e6d9dcea1696db95a` (linux/amd64 `sha256:b121b052b4f3b5c88ca8c36d4801c2ffef6e709760d848c84d6a7d5f450e9657`, linux/arm64 `sha256:23f5db58cd4d1ec28a75bf98ff7799b0d05c03ad345a1d59ac4ee142df4fb02e`) — both under `gsoci.azurecr.io/giantswarm/`, cosign-signed with the org's CircleCI identity, SPDX SBOM and provenance attached, Trivy clean, mirrored to the China registry; no charts (see "Publishing"). The first dev build of the consumed branch, `v1.5.1-dev.giantswarm.2026-09-19.05-51-20.hf72ac66`, landed the same way |
| **v1.5.1-gs.4** (2026-09-14, tag on `86975ac2` = upstream `main` `c1d24607` + the resuming-actor admission (`9a4c4731`) + the `GRPCRoute` method-match translation (`8990e17c`) + the fork's files; [run 34890123667](https://github.com/giantswarm/agentgateway-upstream/actions/runs/34890123667)) | `main` @ `c1d24607` | `agentgateway:v1.5.1-gs.4` `sha256:f3d4b52c53badc23253fe51b17d655865000ec49fc3614c26a59d63b6f540e6c` · `controller:v1.5.1-gs.4` `sha256:f9db28ed2c765ef352d451bae0ae1b893563ab9a9aec5fc00cfbbd6e8babc240` (both linux/amd64 + linux/arm64, cosign-signed). **Images only**: the run's controller scan failed on the four Istio pseudo-version advisories (CVE-2019-14993, CVE-2021-39155, CVE-2021-39156, CVE-2022-23635 — the time-boxed false positives above) because their `.trivyignore` entries had been added by the dependency-bump commit the re-pin dropped, so the `charts` job was skipped and no `1.5.1-gs.4` chart exists; the data-plane image scan was clean. The entries are restored by pull request #9; the line's next release publishes charts again. No consumer reads the line's charts (the packaging chart vendors upstream's) |
| **v1.5.1-gs.3** (2026-09-11, tag on `2c8cacbb` = v1.5.1-gs.2 + the `GRPCRoute` method-match translation fix (pull request #6: a service-only match is a path prefix, `type: RegularExpression` is honoured); [run 34550503402](https://github.com/giantswarm/agentgateway-upstream/actions/runs/34550503402)) | v1.5.0 | `agentgateway:v1.5.1-gs.3` `sha256:98ea7da34357ab09c933d86c0421c77860131a05af664f74958c8d6b3043e929` · `controller:v1.5.1-gs.3` `sha256:0a4e033a7b6dcfad8dd2749c118206bc8090fe8bb9ab3e03b6876aee320deee0` (both linux/amd64 + linux/arm64, cosign-signed, Trivy clean) · charts `agentgateway:1.5.1-gs.3` `sha256:1312f4056d9f1185b396836f942bbc03e43e90ebc4dfa67431c95aa547ac194f`, `agentgateway-crds:1.5.1-gs.3` `sha256:1eb97f878f8eced81518ab2e4e03100c83613094f567ff759e4ab2237ab00943`, `agentgateway-standalone:1.5.1-gs.3` `sha256:57d48da620964180508326da17fb708c2e1a502218065f5bff62fcf4c96c2d11` (the `v1.5.1-gs.3` chart tags carry the same content under a second manifest) |
| **v1.5.1-gs.2** (2026-09-11, tag on `4d34ff55` = v1.5.0 + the grpc/x/crypto bump + agentgateway#3237's CONNECT-time actor egress authorization + the resuming-actor admission; [run 34544951234](https://github.com/giantswarm/agentgateway-upstream/actions/runs/34544951234)) | v1.5.0 | `agentgateway:v1.5.1-gs.2` `sha256:766f68bc1ec30c122615cd7d35d15ecada873f4a2623ff1b61fff67724145132` · `controller:v1.5.1-gs.2` `sha256:f0c7539b1b117ae4883f5de713f417d39b813c0f772c9d66c3890b1ba825ebfa` (both linux/amd64 + linux/arm64, cosign-signed, Trivy clean) · charts `agentgateway:1.5.1-gs.2` `sha256:af5b06c75db1c0b65fa79fd77a74ea5471c4f57c577fb39063b5e96761b7d1f2`, `agentgateway-crds:1.5.1-gs.2` `sha256:6bc86f889a512f06f12129580307704c66a59370081106561c5901b543cdb25f`, `agentgateway-standalone:1.5.1-gs.2` `sha256:c6868e6a2bd16e36dc3edb5769ad892728cda8378e79f6fbfa09755202c08c48` (the `v1.5.1-gs.2` chart tags carry the same content under a second manifest) |
| **v1.5.1-gs.1** (2026-09-10, tag on `5a5d5d8b` = v1.5.0 + the grpc/x/crypto bump; [run 34541814717](https://github.com/giantswarm/agentgateway-upstream/actions/runs/34541814717)) | v1.5.0 | `agentgateway:v1.5.1-gs.1` `sha256:e100bc9aea668ce0cc1c178057a34f1794f24a168d98d6da1144cfd4d4b0b7ee` · `controller:v1.5.1-gs.1` `sha256:78225d54d6e624582dc208eaa80fa89339da0346395bf67e38f40f5ae04b62e8` (both linux/amd64 + linux/arm64, cosign-signed, Trivy clean) · charts `agentgateway:1.5.1-gs.1` `sha256:60769399af5cfb479764b14054edc133a2c174d8762e726e9b905c3406230c3e`, `agentgateway-crds:1.5.1-gs.1` `sha256:d12a7166cdd8924cdcce632693896187050089bcda42cdc1ef704dd93f48c4da`, `agentgateway-standalone:1.5.1-gs.1` `sha256:08c33dcdb78521490a4075c4c9acd4fc911b0d093f0b34973d398bae90d0d79f` (the `v1.5.1-gs.1` chart tags carry the same content under a second manifest) |

**Scans.** Both own images are scanned with Trivy (HIGH and CRITICAL, fixable only) after the push (`scan-agentgateway`, `scan-controller`: the repo-owned `scan` job runs Trivy as a container on the job's remote Docker host, with `.trivyignore` copied into it). A fixable finding fails the pipeline — the images exist in the registry by then, a red release is
not consumed: bump the dependency (upstream first) or, when upstream has no fix, add a time-boxed entry to `.trivyignore` (`CVE-… exp:YYYY-MM-DD # reason, tracking
issue`) — an expired entry fails again and is re-triaged, not extended. Trivy reports four Istio advisories
(CVE-2019-14993, CVE-2021-39155, CVE-2021-39156, CVE-2022-23635) against `istio.io/istio`, which the controller imports at
a `v0.0.0-<date>` pseudo-version of a 2026 commit: Trivy cannot order a pseudo-version against the advisories' fixed
releases (Istio ≤ 1.13) and flags code that is years past them; they are time-boxed in `.trivyignore` until 2026-12-31
(re-triage: an `istio.io/istio` release tag in `go.mod`, or Trivy learning pseudo-versions). The controller's Go module
graph is covered by `govulncheck` on every push and pull request. Trivy does not see into a Rust binary that was not built with
`cargo auditable`; upstream's `deny.toml` (`cargo deny check advisories`) is the tool for the Rust dependency graph
and is not wired into CI yet — a known gap of the line, tracked in #37758.

**Builds are cold.** The Rust build runs inside the Dockerfile with BuildKit cache mounts that a fresh remote-Docker VM
does not persist, so every publish rebuilds from scratch (the build jobs run on `xlarge` / `arm.xlarge`). Good enough
for a line that rebuilds on re-pins and patches; a persisted cargo cache is an improvement, not a prerequisite.

## Consumers

| Consumer | Where the pin lives | Selects |
|---|---|---|
| [giantswarm/substrate](https://github.com/giantswarm/substrate) (the Substrate line) | `.github/workflows/publish.yaml` `AGENTGATEWAY_IMAGE`, stamped into the chart's `images.agentgateway` at package time | the line's release `gsoci.azurecr.io/giantswarm/agentgateway-upstream/agentgateway:<version>` for `atenet-router` and `atenet-egress`; the move from the flattened name and its `v`-prefixed tag is the Substrate line's own pull request |
| [giantswarm/retagger](https://github.com/giantswarm/retagger) | `images/renamed-agentgateway.yaml` | copies **upstream's** releases (`cr.agentgateway.dev`) into the flattened `gsoci.azurecr.io/giantswarm/agentgateway` and `…/agentgateway-controller`; nothing of the line's (its copies of the line's ghcr releases are retired, [giantswarm/retagger#1238](https://github.com/giantswarm/retagger/issues/1238)) |
| [giantswarm/agentgateway](https://github.com/giantswarm/agentgateway) (the packaging chart) | `sync/patches/values/values.yaml` → `helm/agentgateway/values.yaml`: `controller.image` and `proxy.image` (repository and tag; Renovate follows the stable tags) | a release from `gsoci.azurecr.io/giantswarm/agentgateway-upstream/{controller,agentgateway}` — the move from the flattened names and the `-gs.` cap is the packaging chart's own pull request; its chart stays upstream's, vendored from `cr.agentgateway.dev` until the line carries a chart patch (then `vendir.yml` points at this registry's charts) |
| [giantswarm/agent-platform](https://github.com/giantswarm/agent-platform) | the meta chart follows the packaging chart through `components.agentgateway.versionRange`; the connectivity chart pins the data-plane image in `agentgateway.proxy.image` (repository and tag, rendered into `AgentgatewayParameters`) | the same release |

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
  lint, controller e2e and both conformance suites on kind — plus `govulncheck`; `ci-ok` is required. The CircleCI pipeline publishes only from `giantswarm` and release tags; on a pull-request branch it runs the `validate` workflow — the same image builds with `push: false`, about as long as the Rust lanes. Expect an hour: the Rust lanes build on 4-vCPU hosted runners with caches that
  only pushes to `giantswarm` roll forward.
- Upstream's `release.yml` and `nightly.yml` cannot be dispatched here (their `workflow_dispatch` triggers are
  removed on this branch; they would push under upstream's names to ghcr.io). Do not push tags other than the line's releases (`vX.Y.Z`, `vX.Y.Z-rc.N`; see "Publishing").
