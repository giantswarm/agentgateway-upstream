#!/bin/sh
# Fork (giantswarm/agentgateway-upstream): every carried-change file under the ledger
# directory opens with the YAML front-matter `fork-ledger validate` reads
# (FORK.md, the ledger; giantswarm/kagent-upstream#719). This check is the
# same rules in POSIX sh and awk, so CI needs no Go module to run it.
#
#   .github/scripts/ledger-header.sh <fork> <dir>                              # the headers
#   .github/scripts/ledger-header.sh --complete [--upstream-ref <ref>] <fork> <dir>   # and every commit named once
#
# One line per problem on stderr, exit 1 on any.
#
# A header is the block between the opening `---` on line 1 and the next
# `---`: `id` is the file name without .md, `fork` is <fork>, `theme` is one
# of the twenty shared slugs, `category`, `status` and `upstream.outcome` take
# their listed values, `commits` lists the carried commit subjects (non-empty
# for a carried change; a trailing " (#N)" is never part of a subject),
# `upstream.refs` are `owner/repo#N` and never name giantswarm/giantswarm,
# `depends_on` entries are such a reference, a file of this ledger, or
# `owner/repo:<id>` in another line's ledger, `verified` is a date,
# `upstream.exit` is set unless the outcome is merged or none, the body has a
# `## History` section and none of the first-generation bullets, and no
# `commit` key remains. No subject is named by two files.
#
# `--complete` adds the reverse direction: every non-merge commit between the
# merge base with the upstream mirror (`--upstream-ref`, default origin/main)
# and HEAD whose subject no prefix of <dir>/EXEMPT covers is named by exactly
# one file. The mirror ref must be fetched.
set -eu

complete=0
upstream_ref=origin/main
while [ "$#" -gt 0 ]; do
  case "$1" in
    --complete) complete=1; shift ;;
    --upstream-ref) upstream_ref=$2; shift 2 ;;
    --) shift; break ;;
    -*) echo "unknown option $1" >&2; exit 64 ;;
    *) break ;;
  esac
done
if [ "$#" -ne 2 ]; then
  echo "usage: $0 [--complete] [--upstream-ref <ref>] <fork> <dir>" >&2
  exit 64
fi
fork=$1
dir=$2
if [ ! -d "$dir" ]; then
  echo "$dir: not a directory" >&2
  exit 64
fi

ids=$(for f in "$dir"/*.md; do b=${f##*/}; [ "$b" = README.md ] && continue; printf '%s\n' "${b%.md}"; done)
subjects=$(mktemp)
trap 'rm -f "$subjects"' EXIT

problems=0
for f in "$dir"/*.md; do
  base=${f##*/}
  [ "$base" = README.md ] && continue
  if ! out=$(awk -v fork="$fork" -v id="${base%.md}" -v ids="$ids" '
    function unquote(v,    q) {
      sub(/^[ \t]+/, "", v); sub(/[ \t]+$/, "", v)
      q = substr(v, 1, 1)
      if ((q == "\"" || q == "'"'"'") && substr(v, length(v), 1) == q && length(v) >= 2) {
        v = substr(v, 2, length(v) - 2)
        if (q == "'"'"'") gsub(/'"''"'/, "'"'"'", v)
        else gsub(/\\"/, "\"", v)
      }
      return v
    }
    function problem(m) { print m; problems++ }
    function islist(v) { return v ~ /^\[.*\]$/ }
    function items(v, arr,    n, i, raw) {
      v = substr(v, 2, length(v) - 2)
      if (v ~ /^[ \t]*$/) return 0
      n = split(v, raw, /,[ \t]*/)
      for (i = 1; i <= n; i++) arr[i] = unquote(raw[i])
      return n
    }
    BEGIN {
      split("session-lifecycle workspace-volumes credential-injection durable-pause egress-policies providers-vertex harness-variety golden-boot identity-authorizer mcp-forwarder jwks tenancy compat-migrations chart-knobs chart-shape ci-publish-infra test-flakes version-pins new-rpcs never-filed", t, " ")
      for (i in t) themes[t[i]] = 1
      split("pin infra bugfix feature gs-specific compat test", c, " "); for (i in c) categories[c[i]] = 1
      split("carried dropped superseded", s, " "); for (i in s) statuses[s[i]] = 1
      split("merged open declined superseded to-file fork-only drops-at-repin none", o, " "); for (i in o) outcomes[o[i]] = 1
      n = split(ids, idlist, "\n"); for (i = 1; i <= n; i++) known[idlist[i]] = 1
      problems = 0; closed = 0; in_upstream = 0; in_commits = 0; ncommits = 0; history = 0
    }
    NR == 1 { if ($0 != "---") { problem("no header: the file must start with a --- front-matter"); exit } next }
    closed {
      if ($0 == "## History") history = 1
      if ($0 ~ /^- \*\*(Status|Upstream|Our commit):\*\*/) problem("body: the bullet \"" $0 "\" belongs in the header or under ## History")
      if ($0 ~ /^\| Carried commit \(subject\)/) problem("body: the table belongs in the header and under ## History")
      next
    }
    $0 == "---" { closed = 1; next }
    in_commits && /^  - / { v = $0; sub(/^  - /, "", v); v = unquote(v); ncommits++; commits[ncommits] = v; next }
    /^[a-z_]+:/ {
      in_upstream = 0; in_commits = 0
      key = $0; sub(/:.*/, "", key)
      val = $0; sub(/^[a-z_]+:/, "", val)
      if (key == "upstream") { in_upstream = 1; seen["upstream"] = 1; next }
      if (key == "commits") { in_commits = 1; seen["commits"] = 1; commits_inline = val; next }
      seen[key] = 1; top[key] = val; next
    }
    in_upstream && /^  [a-z_]+:/ {
      key = $0; sub(/^  /, "", key); sub(/:.*/, "", key)
      val = $0; sub(/^  [a-z_]+:/, "", val)
      seen["upstream." key] = 1; up[key] = val; next
    }
    { problem("header: unexpected line " NR ": " $0) }
    END {
      if (NR == 0) { problem("no header: the file is empty"); print "PROBLEMS=" problems; exit }
      if (problems && !closed && NR == 1) { print "PROBLEMS=" problems; exit }
      if (!closed) problem("no header: the front-matter never closes with ---")
      split("id fork theme category status commits upstream.outcome verified", req, " ")
      for (i in req) if (!(req[i] in seen)) problem("header: " req[i] " is missing")
      if ("commit" in seen) problem("header: commit is no longer a field; commits lists the subjects")
      v = unquote(top["id"]); if (("id" in seen) && v != id) problem("id \"" v "\" does not match the file name \"" id "\"")
      v = unquote(top["fork"]); if (("fork" in seen) && v != fork) problem("fork \"" v "\" is not \"" fork "\"")
      v = unquote(top["theme"]); if (("theme" in seen) && !(v in themes)) problem("theme \"" v "\" is not in the catalogue")
      v = unquote(top["category"]); if (("category" in seen) && !(v in categories)) problem("category \"" v "\" is not pin, infra, bugfix, feature, gs-specific, compat or test")
      status = unquote(top["status"]); if (("status" in seen) && !(status in statuses)) problem("status \"" status "\" is not carried, dropped or superseded")
      outcome = unquote(up["outcome"]); if (("upstream.outcome" in seen) && !(outcome in outcomes)) problem("upstream.outcome \"" outcome "\" is not merged, open, declined, superseded, to-file, fork-only, drops-at-repin or none")
      v = unquote(top["verified"]); if (("verified" in seen) && v !~ /^[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]$/) problem("verified \"" v "\" is not a YYYY-MM-DD date")
      if ("commits" in seen) {
        ci = unquote(commits_inline)
        if (ci != "" && ci != "[]") problem("commits must be a block list, one \"  - subject\" per line")
        if (status == "carried" && ncommits == 0) problem("commits is empty while status is carried")
        for (i = 1; i <= ncommits; i++) {
          if (commits[i] == "") problem("commits: an empty subject")
          else if (commits[i] ~ / \(#[0-9]+\)$/) problem("commits \"" commits[i] "\" keeps its pull request suffix")
          else print "SUBJECT=" commits[i]
        }
      }
      if ("upstream.refs" in seen) {
        v = unquote(up["refs"])
        if (!islist(v)) problem("upstream.refs is not a [list]")
        else { n = items(v, refs); for (i = 1; i <= n; i++) {
          if (refs[i] !~ /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+#[0-9]+$/) problem("upstream.refs \"" refs[i] "\" is not owner/repo#N")
          else if (refs[i] ~ /^giantswarm\/giantswarm#/) problem("upstream.refs \"" refs[i] "\" names the internal tracker")
        } }
      }
      exitv = unquote(up["exit"])
      if ((exitv == "" || exitv == "-") && outcome != "merged" && outcome != "none") problem("upstream.exit is empty while outcome is " outcome)
      if ("depends_on" in seen) {
        v = unquote(top["depends_on"])
        if (!islist(v)) problem("depends_on is not a [list]")
        else { n = items(v, deps); for (i = 1; i <= n; i++) {
          d = deps[i]
          if (d ~ /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+#[0-9]+$/) { if (d ~ /^giantswarm\/giantswarm#/) problem("depends_on \"" d "\" names the internal tracker"); continue }
          if (d ~ /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+:[a-z0-9][a-z0-9-]*$/) continue
          if (d ~ /^[a-z0-9][a-z0-9-]*$/) { if (!(d in known)) problem("depends_on \"" d "\" names no file in this ledger"); continue }
          problem("depends_on \"" d "\" is neither owner/repo#N nor [owner/repo:]id")
        } }
      }
      if (!history) problem("body: no ## History section")
      print "PROBLEMS=" problems
    }
  ' "$f") || true; then :; fi
  count=$(printf '%s\n' "$out" | sed -n 's/^PROBLEMS=//p' | tail -1)
  printf '%s\n' "$out" | sed -n "s/^SUBJECT=/${base%.md}	/p" >>"$subjects"
  printf '%s\n' "$out" | grep -v '^PROBLEMS=\|^SUBJECT=' | sed "s|^|$f: |" >&2 || true
  [ -z "$count" ] && count=1
  problems=$((problems + count))
done

# A subject named by two files.
dups=$(cut -f2- "$subjects" | sort | uniq -d)
if [ -n "$dups" ]; then
  printf '%s\n' "$dups" | while IFS= read -r s; do
    files=$(awk -F'\t' -v s="$s" '$2 == s { printf "%s ", $1 }' "$subjects")
    echo "$dir: the subject \"$s\" is named by $files" >&2
  done
  problems=$((problems + $(printf '%s\n' "$dups" | wc -l)))
fi

if [ "$complete" -eq 1 ]; then
  if ! base=$(git merge-base "$upstream_ref" HEAD 2>/dev/null); then
    echo "$dir: cannot find the merge base of $upstream_ref and HEAD; fetch the mirror" >&2
    exit 1
  fi
  exempt=$(sed -n '/^[^#]/p' "$dir/EXEMPT" 2>/dev/null | sed 's/[ \t]*$//')
  missing=$(git log --no-merges --format='%h %s' "$base..HEAD" | awk -v exempt="$exempt" -v subjects="$subjects" '
    BEGIN {
      ne = split(exempt, ex, "\n")
      while ((getline line < subjects) > 0) { sub(/^[^\t]*\t/, "", line); named[line] = 1 }
    }
    {
      sha = $1; subject = $0; sub(/^[^ ]+ /, "", subject); sub(/ \(#[0-9]+\)$/, "", subject)
      for (i = 1; i <= ne; i++) if (ex[i] != "" && index(subject, ex[i]) == 1) next
      if (!(subject in named)) print "commit " sha " \"" subject "\" is named by no file"
    }')
  if [ -n "$missing" ]; then
    printf '%s\n' "$missing" | sed "s|^|$dir: |" >&2
    problems=$((problems + $(printf '%s\n' "$missing" | wc -l)))
  fi
fi

if [ "$problems" -gt 0 ]; then
  echo "$problems problem(s) in $dir" >&2
  exit 1
fi
