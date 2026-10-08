#!/usr/bin/env bash
# Experiment: gcc PGO of the C code (tree-sitter + grammars), which is 73-82 % of the CPU.
# rustc PGO is not used: it needs llvm-profdata matching rustc's LLVM. Linux + gcc only.
# Usage: TRAIN="dir1 dir2 ..." lint/bench/pgo.sh   (default: part of the CPython stdlib)
# Output: $WORK/bin/botica-pgo. Everything lives under $WORK (default <repo>/.tmp/pgo).
set -euo pipefail

ROOT=$(git rev-parse --show-toplevel)
WORK=${WORK:-$ROOT/.tmp/pgo}
STDLIB=${STDLIB:-/usr/lib/python3.14}
TRAIN=${TRAIN:-$(for d in asyncio email http json logging unittest importlib concurrent xml urllib multiprocessing; do echo "$STDLIB/$d"; done)}
export CARGO_TARGET_DIR=$WORK/target CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback
mkdir -p "$WORK/data" "$WORK/bin"

printf '#!/bin/sh\nexec cc "$@" -fprofile-generate\n' >"$WORK/cc-gen.sh"
chmod +x "$WORK/cc-gen.sh"

echo "== 1. instrumented build =="
rm -rf "$WORK/data"/*
(cd "$ROOT/lint" && CFLAGS="-fprofile-generate=$WORK/data -fprofile-update=prefer-atomic" \
  CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=$WORK/cc-gen.sh cargo build --release --quiet)

echo "== 2. training (1 thread) =="
for dir in $TRAIN; do
  (cd "$dir" && RAYON_NUM_THREADS=1 "$WORK/target/release/botica" lint . >/dev/null 2>&1 || true)
done
find "$WORK/data" -name '*.gcda' | wc -l

echo "== 3. optimized build =="
(cd "$ROOT/lint" && CFLAGS="-fprofile-use=$WORK/data -fprofile-correction -Wno-missing-profile -Wno-coverage-mismatch" \
  cargo build --release --quiet)
cp "$WORK/target/release/botica" "$WORK/bin/botica-pgo"
ls -la "$WORK/bin/botica-pgo"
