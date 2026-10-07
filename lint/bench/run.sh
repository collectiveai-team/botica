#!/usr/bin/env bash
# Reproduces the botica lint measurement spike. Requires: cargo, uv, hyperfine, ast-grep, python3.
# Everything is written under $WORK (default: <repo>/.tmp/spike); nothing is committed.
set -euo pipefail

ROOT=$(git rev-parse --show-toplevel)
WORK=${WORK:-$ROOT/.tmp/spike}
HYPERFINE=${HYPERFINE:-hyperfine}
BOTICA=$ROOT/lint/target/release/botica
BENCH=$ROOT/lint/bench
OUT=$WORK/results
mkdir -p "$OUT" "$WORK/hooks" "$WORK/sg/rules"

(cd "$ROOT/lint" && cargo build --release --quiet)

[ -d "$WORK/repo10k" ] || python3 "$BENCH/gen_repo.py" "$WORK/repo10k" 10000
[ -d "$WORK/repo10k-clean" ] || python3 "$BENCH/gen_repo.py" "$WORK/repo10k-clean" 10000 clean

cp "$ROOT"/scaffolding/templates/ast-grep-rules/*.yml "$ROOT/lint/crates/botica-cli/rules/no-console-log.yml" "$WORK/sg/rules/"
printf 'ruleDirs:\n  - rules\n' >"$WORK/sg/sgconfig.yml"

cat >"$WORK/hooks/fsg.sh" <<'EOF'
#!/bin/sh
exec python3 -c "import sys; sys.exit(max([0] + [1 for f in sys.argv[1:] for n in [sum(1 for _ in open(f, errors='ignore'))] if (n >= 700 and not print(f'Error: CES-71 (file-size-guard): {f} is too large ({n} lines, limit is 700)')) or (n >= 400 and not print(f'Warning: CES-71 (file-size-guard): {f} is large ({n} lines, recommended limit is 400)') and False)]))" "$@"
EOF
cat >"$WORK/hooks/all_py.sh" <<'EOF'
#!/bin/sh
cd "$1" && find . -name '*.py' -print0 | xargs -0 "$2"
EOF
chmod +x "$WORK"/hooks/*.sh

hf() { "$HYPERFINE" -N --warmup "${WARMUP:-3}" --runs "${RUNS:-20}" --ignore-failure "$@"; }

echo "== parity with the ast-grep CLI =="
python3 "$BENCH/parity.py" "$WORK/repo10k" "$BOTICA" "$WORK/sg" | tee "$OUT/parity.txt"

ONE=$WORK/repo10k/pkg_000/mod_1.py
echo "== cold start and one file =="
hf --export-markdown "$OUT/one-file.md" \
  "$BOTICA --version" \
  "$BOTICA lint $ONE --no-astgrep" \
  "$BOTICA lint $ONE" \
  "$WORK/hooks/fsg.sh $ONE" \
  "ast-grep scan --config $WORK/sg/sgconfig.yml $ONE" \
  "uvx --from ast-grep-cli ast-grep scan --config $WORK/sg/sgconfig.yml $ONE"

echo "== a commit-sized batch (~200 files) =="
(cd "$WORK/repo10k" && hf --export-markdown "$OUT/batch-200.md" \
  "$BOTICA lint pkg_001 pkg_002 --no-astgrep" \
  "$BOTICA lint pkg_001 pkg_002")

echo "== 10k files =="
(cd "$WORK/repo10k" && RUNS=10 hf --export-markdown "$OUT/repo-10k.md" \
  "$BOTICA lint . --no-astgrep" \
  "$BOTICA lint .")
(cd "$WORK/repo10k-clean" && RUNS=10 hf --export-markdown "$OUT/repo-10k-clean.md" "$BOTICA lint .")
(cd "$WORK/sg" && RUNS=5 WARMUP=1 hf --export-markdown "$OUT/repo-10k-ast-grep-cli.md" \
  "ast-grep scan ../repo10k" "ast-grep scan ../repo10k-clean")
RUNS=5 WARMUP=1 "$HYPERFINE" --warmup 1 --runs 5 --ignore-failure --export-markdown "$OUT/repo-10k-python-hook.md" \
  "$WORK/hooks/all_py.sh $WORK/repo10k $WORK/hooks/fsg.sh"

echo "== 10k files, calibrated on real code (gen_realistic.py) =="
[ -d "$WORK/repo10k-real" ] || python3 "$BENCH/gen_realistic.py" "$WORK/repo10k-real" 10000
BOTICA_PREFILTER_REGEX=1 python3 "$BENCH/parity.py" "$WORK/repo10k-real" "$BOTICA" "$WORK/sg" | tee "$OUT/parity-real.txt"
(cd "$WORK/repo10k-real" && RUNS=8 hf --export-markdown "$OUT/repo-10k-real.md" \
  "$BOTICA lint . --no-prefilter" \
  "$BOTICA lint ." \
  "env BOTICA_PREFILTER_REGEX=1 $BOTICA lint ." \
  "env BOTICA_PREFILTER_REGEX=1 BOTICA_SCHED_LARGEST_FIRST=1 $BOTICA lint .")
# Per-file cost concentration: BOTICA_FILE_TIMES=1 prints one `[file]` row per file on stderr.

echo "== prefilter and result cache (10k synthetic files) =="
rm -rf "$WORK/cache-10k"
(cd "$WORK/repo10k" && "$BOTICA" lint . --cache-dir "$WORK/cache-10k" >/dev/null 2>&1 || true)
(cd "$WORK/repo10k" && RUNS=10 hf --export-markdown "$OUT/cache-10k.md" \
  "$BOTICA lint . --no-prefilter" \
  "$BOTICA lint ." \
  "$BOTICA lint . --cache-dir $WORK/cache-10k")

STDLIB=${STDLIB:-/usr/lib/python3.14}
if [ -d "$STDLIB" ]; then
  echo "== real corpus: $STDLIB =="
  python3 "$BENCH/parity.py" "$STDLIB" "$BOTICA" "$WORK/sg" | tee "$OUT/parity-stdlib.txt"
  (cd "$STDLIB" && "$BOTICA" lint . --timings 2>&1 >/dev/null | grep -E "prefilter" || true)
  rm -rf "$WORK/cache-stdlib"
  (cd "$STDLIB" && "$BOTICA" lint . --cache-dir "$WORK/cache-stdlib" >/dev/null 2>&1 || true)
  (cd "$STDLIB" && RUNS=5 WARMUP=1 hf --export-markdown "$OUT/stdlib.md" \
    "$BOTICA lint . --no-prefilter" \
    "$BOTICA lint ." \
    "$BOTICA lint . --cache-dir $WORK/cache-stdlib" \
    "$BOTICA lint . --no-astgrep" \
    "env BOTICA_PREFILTER_REGEX=1 $BOTICA lint .")
  echo "== regex prefilter parity on real code =="
  BOTICA_PREFILTER_REGEX=1 python3 "$BENCH/parity.py" "$STDLIB" "$BOTICA" "$WORK/sg" | tee "$OUT/parity-stdlib-regex.txt"
fi

echo "== thread scaling (10k files, native + ast-grep) =="
for threads in 1 2 4 8 16; do
  (cd "$WORK/repo10k" && RAYON_NUM_THREADS=$threads RUNS=3 WARMUP=1 hf "$BOTICA lint ." | grep -E "Time" | sed "s/^/threads=$threads /")
done

echo "== extension start-up =="
hf --export-markdown "$OUT/ext-startup.md" \
  "$BOTICA lint $ONE --no-astgrep" \
  "$BOTICA lint $ONE --no-astgrep --ext 'python3 $BENCH/ext_noop.py'" \
  "$BOTICA lint $ONE --no-astgrep --ext 'uv run --no-project --script $BENCH/ext_noop.py'" \
  "$BOTICA lint $ONE --no-astgrep --ext 'uv run --script $BENCH/ext_bare_except.py'" \
  "python3 -c pass"

echo "== extension round trip (6 961 Python files, protocol only) =="
for batch in 16 64 256 1024; do
  for _ in 1 2 3; do
    (cd "$WORK/repo10k" && "$BOTICA" lint . --no-astgrep --batch "$batch" \
      --ext "uv run --no-project --script $BENCH/ext_noop.py" --timings 2>&1 >/dev/null |
      grep -E "ext lint" | sed "s/^/batch=$batch /") || true
  done
done

echo "== extension doing real work (6 961 Python files) =="
for ext in ext_stdlib ext_bare_except; do
  (cd "$WORK/repo10k" && "$BOTICA" lint . --no-astgrep --ext "uv run --script $BENCH/$ext.py" --timings 2>&1 >/dev/null |
    grep -E "ext lint" | sed "s/^/$ext /") || true
done

echo "== criterion (in-process) =="
(cd "$ROOT/lint" && cargo bench -p botica-core --bench lint_source -- --warm-up-time 1 --measurement-time 3 2>&1 |
  grep -E "^(lint_source|compile|parse_only)|time:")
