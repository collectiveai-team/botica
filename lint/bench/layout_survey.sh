#!/usr/bin/env bash
# How many findings each layout rule gives on repositories pinned by commit (layout-repos.txt), by
# ecosystem: the evidence for a rule and for the ecosystems it applies to. Run it again to see what
# a change to a rule, or a new ecosystem, does to them.
#
# Usage: layout_survey.sh <botica-binary> [workdir]
# Clones (one commit, no history) go to <workdir>/layout/<name>; a repository that cannot be cloned
# is skipped.
set -uo pipefail
ROOT=$(git rev-parse --show-toplevel)
BENCH=$ROOT/lint/bench
BOTICA=$(realpath "$1")
WORK=${2:-$ROOT/.tmp/bench}
RULES="no-tests-in-package,no-notebooks,no-artifacts-in-package"

printf '%-18s %-7s %6s  %5s %5s %5s   %s\n' repo eco files tests notebk artif "code directories found"
while read -r name url commit eco; do
  case "$name" in '' | '#'*) continue ;; esac
  repo=$WORK/layout/$name
  if [ ! -d "$repo/.git" ]; then
    mkdir -p "$repo"
    (cd "$repo" && git init -q && git remote add origin "$url" && git fetch -q --depth 1 origin "$commit" 2>/dev/null \
      && git checkout -q FETCH_HEAD) \
      || { echo "$name: cannot clone, skipped"; rm -rf "$repo"; continue; }
  fi
  files=$(cd "$repo" && git ls-files | wc -l)
  json=$(cd "$repo" && "$BOTICA" lint . --no-astgrep --select "$RULES" --output-format json 2>/dev/null)
  dirs=$(cd "$repo" && "$BOTICA" explain no-notebooks 2>/dev/null | sed -n 's/^code-dirs: //p' | sed 's/ (detected)//' | cut -c1-70)
  printf '%-18s %-7s %6s  ' "$name" "$eco" "$files"
  printf '%s' "$json" | python3 "$BENCH/layout_tally.py"
  printf '   %s\n' "$dirs"
done <"$BENCH/layout-repos.txt"
