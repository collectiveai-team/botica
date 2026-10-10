#!/usr/bin/env bash
# Parity of botica's embedded ast-grep rules on real repositories pinned by commit (real-repos.txt).
#
# 1. botica against the ast-grep CLI (the same 10 rules), with parity.py;
# 2. botica with --no-regions, --no-prefilter and --no-subtree-skip against botica as it ships:
#    the optimisations must not change what is found (the full output, native rules included).
#
# Usage: parity_real.sh <botica-binary> [workdir]
# Needs ast-grep 0.45.3 on PATH and the rules in <workdir>/sg (bench/run.sh sets that up).
# Clones go to <workdir>/real/<name>; amarc is a private repository, so it is skipped if it
# cannot be cloned.
set -uo pipefail
ROOT=$(git rev-parse --show-toplevel)
BENCH=$ROOT/lint/bench
BOTICA=$(realpath "$1")
WORK=${2:-$ROOT/.tmp/bench}
status=0

while read -r name url commit; do
  case "$name" in '' | '#'*) continue ;; esac
  repo=$WORK/real/$name
  if [ ! -d "$repo/.git" ]; then
    mkdir -p "$WORK/real"
    git clone --quiet "$url" "$repo" || { echo "== $name: cannot clone, skipped"; continue; }
  fi
  git -C "$repo" checkout --quiet "$commit" || { echo "== $name: cannot check out $commit"; status=1; continue; }

  echo "== $name against the ast-grep CLI"
  python3 "$BENCH/parity.py" "$repo" "$BOTICA" "$WORK/sg" | head -2
  [ "${PIPESTATUS[0]}" -eq 0 ] || status=1

  (cd "$repo" && "$BOTICA" lint . >"$WORK/parity-default.out" 2>/dev/null)
  for flag in --no-prefilter --no-regions --no-subtree-skip; do
    (cd "$repo" && "$BOTICA" lint . "$flag" >"$WORK/parity-variant.out" 2>/dev/null)
    if cmp -s "$WORK/parity-default.out" "$WORK/parity-variant.out"; then
      echo "   identical with $flag ($(wc -l <"$WORK/parity-variant.out") lines)"
    else
      echo "   DIFFERENT with $flag"
      diff "$WORK/parity-default.out" "$WORK/parity-variant.out" | head -5
      status=1
    fi
  done
done <"$BENCH/real-repos.txt"
exit $status
