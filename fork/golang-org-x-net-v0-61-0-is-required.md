# golang.org/x/net v0.61.0 is required

- **Status:** carried
- **Upstream:** not for upstream as such: falls away with the row above at the re-pin onto the first upstream commit that requires `x/net` ≥ v0.61.0 (upstream's dependency automation moves it)
- **Our commit:** the `fix(deps): bump golang.org/x/net to v0.61.0 for two HTTP/2 CVEs` commit

## Change

Require `golang.org/x/net` v0.61.0 in `go.mod` and `tools/go.mod` (`go get golang.org/x/net@v0.61.0 && go mod tidy` in each; `x/sync`, `x/term`, `x/text` and `x/tools` move along, and the controller's generated license table `hack/utils/oss_compliance/osa_provided.md` names the new `x/sync` and `x/tools`, which `make verify` checks)

## Reason

CVE-2026-97032 and CVE-2026-78663, two HTTP/2 vulnerabilities in `x/net` below v0.61.0; the platform's Go repositories move together, their CircleCI nancy step refusing the older module
