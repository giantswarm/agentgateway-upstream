#!/usr/bin/env bash
# Fork (giantswarm/agentgateway-upstream): fixture runs of ledger-header.sh, one ledger
# directory per case, plus a throwaway repository for the --complete cases.

set -o errexit -o nounset -o pipefail

CHECK="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/ledger-header.sh"
DIR="$(mktemp -d)"
trap 'rm -rf "${DIR}"' EXIT

failures=0

good() {
  cat <<'EOF'
---
id: a-good-row
fork: agentgateway
theme: session-lifecycle
category: bugfix
status: carried
commits:
  - 'fix(x): the first carried subject'
upstream:
  outcome: open
  refs: [agent-substrate/substrate#1543, giantswarm/agentgateway-upstream#12]
  exit: 'as#1543 merged; the line''s patch drops'
depends_on: [another-row, giantswarm/substrate:let-actors-egress-through-while-resumes, agent-substrate/substrate#2117]
verified: 2026-10-19
---
# A good row

## Change

What it does.

## Reason

Why.

## History

The former Upstream and Our commit text.
EOF
}

# Another row names a second subject, so the two files never collide.
other() {
  good | sed "s/^id: a-good-row\$/id: another-row/; s/the first carried subject/the second carried subject/"
}

# case <name> <want exit code> <file name> <file text> [text the output must name]
case_() {
  local name="$1" want="$2" file="$3" text="$4" names="${5:-}" got=0 out ledger
  ledger="${DIR}/${name// /-}"
  mkdir -p "${ledger}"
  good >"${ledger}/a-good-row.md"
  other >"${ledger}/another-row.md"
  printf '%s\n' "${text}" >"${ledger}/${file}"
  out="$("${CHECK}" agentgateway "${ledger}" 2>&1)" || got=$?
  if [[ "${got}" != "${want}" ]] || [[ -n "${names}" && "${out}" != *"${names}"* ]]; then
    echo "FAIL ${name}: exit ${got}, want ${want}${names:+ naming ${names}}"
    printf '%s\n' "${out}" | sed 's/^/    /'
    failures=$((failures + 1))
  else
    echo "ok   ${name}"
  fi
}

third() { good | sed "s/^id: a-good-row\$/id: third-row/; s/the first carried subject/the third carried subject/"; }

case_ "a good ledger" 0 "third-row.md" "$(third)"
case_ "no header" 1 "bare.md" "# Bare" "no header"
case_ "never closed" 1 "open-ended.md" "$(printf -- '---\nid: open-ended\nfork: agentgateway\n')" "never closes"
case_ "id names another file" 1 "wrong-id.md" "$(third | sed 's/^id: third-row$/id: a-good-row/')" 'does not match the file name "wrong-id"'
case_ "the other fork" 1 "other-fork.md" "$(third | sed 's/^id: third-row$/id: other-fork/; s/^fork: .*/fork: elsewhere/')" 'fork "elsewhere"'
case_ "a theme off the catalogue" 1 "off-theme.md" "$(third | sed 's/^id: third-row$/id: off-theme/; s/^theme: .*/theme: misc/')" 'theme "misc"'
case_ "a category off the list" 1 "off-category.md" "$(third | sed 's/^id: third-row$/id: off-category/; s/^category: .*/category: chore/')" 'category "chore"'
case_ "an outcome off the list" 1 "off-outcome.md" "$(third | sed 's/^id: third-row$/id: off-outcome/; s/^  outcome: .*/  outcome: pending/')" 'upstream.outcome "pending"'
case_ "a missing key" 1 "no-verified.md" "$(third | sed 's/^id: third-row$/id: no-verified/; /^verified:/d')" "verified is missing"
case_ "a bad date" 1 "bad-date.md" "$(third | sed 's/^id: third-row$/id: bad-date/; s/^verified: .*/verified: yesterday/')" 'verified "yesterday"'
case_ "a bare reference" 1 "bare-ref.md" "$(third | sed 's/^id: third-row$/id: bare-ref/; s/^  refs: .*/  refs: [#12]/')" 'upstream.refs "#12"'
case_ "the internal tracker" 1 "tracker.md" "$(third | sed 's/^id: third-row$/id: tracker/; s/^  refs: .*/  refs: [giantswarm\/giantswarm#37742]/')" "internal tracker"
case_ "an exit missing while open" 1 "no-exit.md" "$(third | sed 's/^id: third-row$/id: no-exit/; /^  exit:/d')" "upstream.exit is empty while outcome is open"
case_ "an exit missing while merged" 0 "merged.md" "$(third | sed 's/^id: third-row$/id: merged/; s/^  outcome: .*/  outcome: merged/; /^  exit:/d')"
case_ "a dependency on no file" 1 "no-dep.md" "$(third | sed 's/^id: third-row$/id: no-dep/; s/^depends_on: .*/depends_on: [missing-row]/')" 'depends_on "missing-row" names no file'
case_ "a malformed dependency" 1 "bad-dep.md" "$(third | sed 's/^id: third-row$/id: bad-dep/; s/^depends_on: .*/depends_on: [Not A Row]/')" 'depends_on "Not A Row"'
case_ "no commits while carried" 1 "no-commits.md" "$(third | sed 's/^id: third-row$/id: no-commits/; /^  - .fix(x)/d')" "commits is empty while status is carried"
case_ "no commits while dropped" 0 "dropped.md" "$(third | sed 's/^id: third-row$/id: dropped/; s/^status: .*/status: dropped/; /^  - .fix(x)/d')"
case_ "commits missing" 1 "no-commits-key.md" "$(third | sed 's/^id: third-row$/id: no-commits-key/; /^commits:/d; /^  - .fix(x)/d')" "commits is missing"
case_ "an inline commits list" 1 "inline-commits.md" "$(third | sed 's/^id: third-row$/id: inline-commits/; s/^commits:$/commits: [a subject]/; /^  - .fix(x)/d')" "block list"
case_ "a subject with its pull request suffix" 1 "suffix.md" "$(third | sed 's/^id: third-row$/id: suffix/; s/^  - .fix(x).*/  - "fix(x): suffixed (#12)"/')" "keeps its pull request suffix"
case_ "a subject named twice" 1 "twice.md" "$(third | sed 's/^id: third-row$/id: twice/; s/the third carried subject/the first carried subject/')" 'is named by'
case_ "the old commit key" 1 "old-commit.md" "$(third | sed 's/^id: third-row$/id: old-commit/; s/^verified:/commit: 52f1c49a\nverified:/')" "commit is no longer a field"
case_ "a Status bullet" 1 "bullet.md" "$(third | sed 's/^id: third-row$/id: bullet/; s/^## Change$/- **Status:** carried\n\n## Change/')" "belongs in the header"
case_ "no History" 1 "no-history.md" "$(third | sed 's/^id: third-row$/id: no-history/; /^## History/,$d')" "no ## History section"
case_ "a subject the YAML quoting ends early" 1 "bad-quote.md" "$(third | sed "s/^id: third-row\$/id: bad-quote/; s/^  - .fix(x).*/  - 'fix(x): the line's subject'/")" "not valid YAML"

# --complete: a throwaway repository with a mirror branch and three carried commits.
repo="${DIR}/repo"
git init -q "${repo}"
git -C "${repo}" -c user.name=t -c user.email=t@example.com -c commit.gpgsign=false commit -q --allow-empty -m "upstream: base"
git -C "${repo}" branch -q mirror
git -C "${repo}" -c user.name=t -c user.email=t@example.com -c commit.gpgsign=false commit -q --allow-empty -m "fix(x): the first carried subject (#1)"
git -C "${repo}" -c user.name=t -c user.email=t@example.com -c commit.gpgsign=false commit -q --allow-empty -m "fix(x): the second carried subject"
git -C "${repo}" -c user.name=t -c user.email=t@example.com -c commit.gpgsign=false commit -q --allow-empty -m "docs(fork): a ledger edit"
git -C "${repo}" -c user.name=t -c user.email=t@example.com -c commit.gpgsign=false commit -q --allow-empty -m "feat(y): an unlisted change"
mkdir -p "${repo}/fork"
good >"${repo}/fork/a-good-row.md"
other >"${repo}/fork/another-row.md"
printf '# comment\ndocs(\n' >"${repo}/fork/EXEMPT"

complete_case() {
  local name="$1" want="$2" names="${3:-}" got=0 out
  out="$(cd "${repo}" && "${CHECK}" --complete --upstream-ref mirror agentgateway fork 2>&1)" || got=$?
  if [[ "${got}" != "${want}" ]] || [[ -n "${names}" && "${out}" != *"${names}"* ]]; then
    echo "FAIL ${name}: exit ${got}, want ${want}${names:+ naming ${names}}"
    printf '%s\n' "${out}" | sed 's/^/    /'
    failures=$((failures + 1))
  else
    echo "ok   ${name}"
  fi
}

complete_case "an unlisted commit" 1 'commit'
complete_case "an unlisted commit names its subject" 1 '"feat(y): an unlisted change" is named by no file'
printf 'feat(y)\n' >>"${repo}/fork/EXEMPT"
complete_case "an exempt prefix covers it" 0
printf -- '---\nid: an-extra\nfork: agentgateway\ntheme: session-lifecycle\ncategory: bugfix\nstatus: carried\ncommits:\n  - "fix(x): the first carried subject"\nupstream:\n  outcome: none\n  refs: []\n  exit: ""\ndepends_on: []\nverified: 2026-10-19\n---\n# An extra\n\n## History\n\nx\n' >"${repo}/fork/an-extra.md"
complete_case "a subject named by two files" 1 'is named by a-good-row an-extra'

if [[ "${failures}" -gt 0 ]]; then
  echo "${failures} case(s) failed"
  exit 1
fi
