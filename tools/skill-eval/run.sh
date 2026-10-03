#!/usr/bin/env bash
# Full 3-layer measurement pipeline for one skill.
#
#   Layer 1 (structural): agnix static lint of SKILL.md.
#   Layer 2 (agent-rubric): blind holistic comparison of with_skill vs
#     without_skill outputs (comparator.py), adapted from skill-creator's
#     comparator agent.
#   Layer 3 (behavioral/outcome): with_skill vs without_skill runs through a
#     real OpenCode agent (executor.py), graded per-expectation against the
#     agentskills.io eval standard (grader.py), then aggregated into
#     benchmark.json/benchmark.md (vendored aggregate_benchmark.py).
#
# See README.md for the full design rationale.
#
# Usage: ./run.sh <path-to-skill> [workspace-dir] [iteration]
set -euo pipefail

SKILL=${1:?"Usage: run.sh <skill-path> [workspace] [iteration]"}
SKILL=$(cd "$SKILL" && pwd)
NAME=$(basename "$SKILL")
cd "$(dirname "$0")"
WORKSPACE=${2:-"./workspace/$NAME"}
ITER=${3:-1}
ITER_DIR="$WORKSPACE/iteration-$ITER"

mkdir -p "$WORKSPACE"

echo "== Layer 1: agnix (static lint) =="
mkdir -p "$ITER_DIR"
npx agnix --format json "$SKILL" | tee "$ITER_DIR/agnix.json"

echo
echo "== Layer 3a: executor (OpenCode-driven with_skill vs without_skill runs) =="
python3 executor.py --skill "$SKILL" --workspace "$WORKSPACE" --iteration "$ITER"

echo
echo "== Layer 3b: grader (LLM judge against evals/evals.json expectations) =="
python3 grader.py --workspace "$WORKSPACE" --iteration "$ITER"

echo
echo "== Layer 3c: aggregate benchmark =="
python3 vendor/skill-creator/scripts/aggregate_benchmark.py "$ITER_DIR" --skill-name "$NAME" --skill-path "$SKILL"

echo
echo "== Layer 2: agent-rubric (blind comparator) =="
python3 comparator.py --workspace "$WORKSPACE" --iteration "$ITER"

echo
echo "Done. Results under $ITER_DIR:"
echo "  - agnix.json              (layer 1: static lint)"
echo "  - eval-*/comparison.json  (layer 2: blind rubric comparison)"
echo "  - eval-*/*/run-1/grading.json, benchmark.json, benchmark.md (layer 3)"
