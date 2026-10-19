# The Giant Swarm line of agentgateway

This repository is Team Bumblebee's fork of [agentgateway/agentgateway](https://github.com/agentgateway/agentgateway).
The Giant Swarm Agent Platform runs agentgateway in two places — as the platform's gateway (the controller and the
data-plane proxy the [`giantswarm/agentgateway`](https://github.com/giantswarm/agentgateway) packaging chart
deploys) and inside Agent Substrate (`atenet-router` and `atenet-egress` of the
[`giantswarm/substrate`](https://github.com/giantswarm/substrate) line) — and this fork is where the agentgateway
they run is pinned, built, scanned and published. It answers one question from one place: **which agentgateway are
we running, and why does it differ from upstream?**

Tracking: the fork's own issues are filed in this repository's tracker; the sibling lines built the same way are
kagent ([giantswarm/kagent-upstream](https://github.com/giantswarm/kagent-upstream)) and Substrate
([giantswarm/substrate](https://github.com/giantswarm/substrate)).

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
| Why this one | The line returns to a release tag at the first stable release that contains `8dba3989`, as the previous pin's row promised, and v1.6.0 is that release. The packaging chart `giantswarm/agentgateway` (the platform's controller and data plane) follows this release; its vendored chart can move to upstream's v1.6.0 chart at the same time (see "Consumers"). The Substrate line follows it since its own re-pin past agent-substrate/substrate#1809 (`7172778d` on giantswarm/substrate, 2026-10-05, the router and egress on `2.3.0`; its chart pins `2.3.5` on 2026-10-13): a release on this pin refuses the CONNECT of an actor whose certificate carries only the old `spiffe://substrate-actor.local/atespace/<atespace>/actor/<actor>` URI, which #3677 no longer reads. |
| When it moves | with the consumers, proven in agentlab first — see "Re-pin". Not on a schedule. Previous pins: v1.5.0 (2026-09-10 → 2026-09-14, releases `v1.5.1-gs.1`–`gs.3`), `main` @ `c1d24607` (2026-09-14 → 2026-09-22, releases `v1.5.1-gs.4`, `v1.5.1-gs.5`, `2.0.0`), `main` @ `08f4563b` (2026-09-22 → 2026-09-24, releases `2.1.0`, `2.1.1`, `2.1.2`), `main` @ `3528a428` (2026-09-24 → 2026-10-05, releases `2.2.0`, `2.2.1`, `2.2.2`, `2.2.3-rc.1`); this pin's releases start at `2.3.0`. |
| Derived how | The pin itself is `git merge-base giantswarm main` (the mirror — after a re-pin onto an upstream commit newer than the last mirror run, `main` lags the pin until the next weekly run and the merge-base is the older commit; this row is authoritative). The upstream release it corresponds to is `git describe --tags --abbrev=0 --match 'v[0-9]*' --exclude '*-*' "$(git merge-base giantswarm main)"` with upstream's tags fetched — the nearest upstream release at or below the pin, v1.6.0 today. `git describe` runs at the merge-base, not at `giantswarm`, so the line's own stable tags (`v2.0.0`, …; see "Publishing") never count. The line's versions are decoupled from upstream's; the pin is documented here and stamped on every published index as the annotation `io.giantswarm.upstream.version`. |

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

Nineteen carried changes sit on the pin, each with a ledger file in `fork/`, beside the file for the fork infrastructure (the Kubernetes JWKS refetch is four commits, offered upstream as one). Ten change agentgateway's behaviour: the admission of a resuming
actor, the file resources read from disk on every reload together with the parse-time files made managed dependencies,
the resume refusal answered at once (#29, which leaves with Substrate's directive), the unreachable policy service
answered as unavailable (#35), a credential provider's failure answered by its class (#39), the credential header applied by request path prefix,
the JWKS refetch on an unknown key id for local configuration (agentgateway#3037) and under Kubernetes, and the gRPC status of a trailers-only refusal in the access log.
Checked against upstream `main` @ `c0354ea4` on 2026-10-13, they fall in four groups. Already upstream after the pin, so they drop at the re-pin onto a commit
containing all three: the parse-time files (agentgateway#3863, `0a8476fe`) with the read on every reload, a workaround that leaves with it, the
unreachable policy service (agentgateway#3793, `0c62b5a3`) and the admission of a resuming actor (agentgateway#3882, `9d36620d`). Filed or to file:
the gRPC status in the access log (agentgateway#3873, merged 2026-10-13 as `1370236e`, so it drops at the re-pin too; the request limit refusal not filed), the credential provider's failure class
(agentgateway#3875, draft, held on Substrate's credential provider contract) and the credential header by path prefix (after agent-substrate/substrate#2515).
Waiting on others: the JWKS refetch for local configuration (agentgateway#3037), and under Kubernetes an issue that is not filed yet. Not for upstream as is: the
resume refusal answered at once, whose directive Substrate 1.3 and, since giantswarm/substrate#445, the line's ate-api send, and whose proper fix is Substrate's codes. The grpc pin at v1.83.2, the Go 1.27.2 build (its `x/net` v0.60.0 half is upstream) and the `x/net` v0.61.0 requirement change versions only and leave when upstream requires fixed releases (see their files in `fork/`).

Dropped at the 2026-10-05 re-pin because the pin contains their fixes: the `GRPCRoute` method-match translation (`223b80e2`, pull request #6;
agentgateway#3723 `2353a2c1`, merged 2026-10-01, closes our agentgateway#3715 and makes a service-only match
a `PathPrefix` and a `RegularExpression` match one path regex) and the e2e warm step's absolute `-toolexec` path (`a2a16cea`,
pull request #21; agentgateway#3704 `f6ceca24`, merged 2026-09-30, resolves the helper from the repository root
the same way, so branch `upstream/ci-toolexec-absolute` is obsolete). Both conflicted at the rebase and were skipped.
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
   `Upstream` field of every ledger file the new pin changes.
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
into gsoci are retired). The
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
'^https://circleci\\.com/api/v2/projects/[a-f0-9-]+/pipeline-definitions/[a-f0-9-]+$'` for the signature). The
line's releases and what each carries are its [GitHub releases](https://github.com/giantswarm/agentgateway-upstream/releases);
`devctl release wait giantswarm/agentgateway-upstream <tag>` prints a release's index digests and its tag pipeline.
Releases up to v1.5.1-gs.4 were published to `ghcr.io/giantswarm/agentgateway-upstream` by the fork workflow and
copied to the flattened gsoci names by retagger, v1.5.1-gs.5 to the flattened names from CircleCI; their ghcr
originals are not maintained.

**Scans.** Both own images are scanned with Trivy (HIGH and CRITICAL, fixable only) after the push (`scan-agentgateway`, `scan-controller`: the repo-owned `scan` job runs Trivy as a container on the job's remote Docker host, with `.trivyignore` copied into it). A fixable finding fails the pipeline — the images exist in the registry by then, a red release is
not consumed: bump the dependency (upstream first) or, when upstream has no fix, add a time-boxed entry to `.trivyignore` (`CVE-… exp:YYYY-MM-DD # reason, tracking
issue`) — an expired entry fails again and is re-triaged, not extended. Trivy reports four Istio advisories
(CVE-2019-14993, CVE-2021-39155, CVE-2021-39156, CVE-2022-23635) against `istio.io/istio`, which the controller imports at
a `v0.0.0-<date>` pseudo-version of a 2026 commit: Trivy cannot order a pseudo-version against the advisories' fixed
releases (Istio ≤ 1.13) and flags code that is years past them; they are time-boxed in `.trivyignore` until 2026-12-31
(re-triage: an `istio.io/istio` release tag in `go.mod`, or Trivy learning pseudo-versions). The controller's Go module
graph is covered by `govulncheck` on every push and pull request. Trivy does not see into a Rust binary that was not built with
`cargo auditable`; upstream's `deny.toml` (`cargo deny check advisories`) is the tool for the Rust dependency graph
and is not wired into CI yet — a known gap of the line.

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

- **Upstream first, prepared here.** Every behavioural change is prepared for upstream
  ([agentgateway/agentgateway](https://github.com/agentgateway/agentgateway); see upstream's `CONTRIBUTION.md`) as one
  upstream-ready commit, DCO-signed (`git commit -s`), with no fork references. The commit goes onto the fork with its
  ledger row in `fork/`, and is pushed as an `upstream/<slug>` branch on this repository. Its ledger row reads "to file —
  prepared in the fork: branch `upstream/<slug>`" until it is opened, and links the upstream pull request from then on.
  A human reviews the prepared branches in batches, decides what goes upstream and opens it; an agent never opens, comments
  on, reviews or closes a pull request or issue on the upstream repository. The line carries the same change as a
  `git cherry-pick -x` of the upstream-ready commit until an upstream release contains it, with its row.
- **Public only.** This repository is public: a ledger row, a commit message and this file cite public issues and pull
  requests only, and where the reasoning lives in a private tracker it is written out in a sentence instead. The
  `public-links` job of the Branch workflow, under `ci-ok`, fails a pull request whose added ledger lines cite a giantswarm
  repository that is not public, or a row of a private list (`.github/scripts/fork-public-links.sh`).
- **Fork-only changes** (workflows, this file): a pull request from `fork/<topic>` against `giantswarm`; squash-merge.
  Cherry-picks of upstream commits are rebase-merged so they keep their patch-id and fall away at the re-pin.
- **Experiments**: your own personal fork. Branches here exist to become pull requests.
- **What CI runs on a pull request**: upstream's `Branch` workflow — proxy tests (Linux, with the Keycloak/JWKS
  validation dependencies), proxy lint (schema generation, clippy), UI lint and Playwright, controller tests and
  lint, controller e2e and both conformance suites on kind — plus `govulncheck` and `public-links`; `ci-ok` is required. The CircleCI pipeline publishes only from `giantswarm` and release tags; on a pull-request branch it runs the `validate` workflow — the same image builds with `push: false`, about as long as the Rust lanes. Expect an hour: the Rust lanes build on 4-vCPU hosted runners with caches that
  only pushes to `giantswarm` roll forward.
- Upstream's `release.yml` and `nightly.yml` cannot be dispatched here (their `workflow_dispatch` triggers are
  removed on this branch; they would push under upstream's names to ghcr.io). Do not push tags other than the line's releases (`vX.Y.Z`, `vX.Y.Z-rc.N`; see "Publishing").
