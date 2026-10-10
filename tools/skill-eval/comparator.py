#!/usr/bin/env python3
"""Layer 2 (agent-rubric): blind holistic comparison of with_skill vs
without_skill outputs for the same eval.

Adapted from vendor/skill-creator/agents/comparator.md (Anthropic,
Apache-2.0): the two outputs are shuffled into anonymous "A"/"B" labels
before being shown to the judge, so it can't be biased by knowing which
config produced which. This complements grader.py's per-expectation PASS/FAIL
grading with a holistic quality rubric (organization, formatting, usability)
-- two outputs can both pass every expectation and still differ in polish.

Writes comparison.json (schema per vendor/skill-creator/references/schemas.md)
into each eval directory, alongside the with_skill/without_skill run dirs.

Usage:
    python3 comparator.py --workspace ./workspace/journalist --iteration 1
"""

import argparse
import json
import random
import sys
from pathlib import Path

from nim_client import chat_json

JUDGE_MODEL = "z-ai/glm-5.2"

COMPARATOR_SYSTEM_PROMPT = """You are the Blind Comparator for an agent-skill evaluation, adapted from \
Anthropic's skill-creator comparator agent. You are shown two candidate outputs, \
labeled A and B, for the SAME task -- you do not know which configuration (with \
or without a skill) produced which. Judge purely on output quality and task \
completion, to avoid bias.

Generate a short rubric for this specific task with two dimensions -- Content \
(correctness, completeness, accuracy) and Structure (organization, formatting, \
usability) -- score each output 1-5 per criterion, and pick an overall winner.

Respond with ONLY a JSON object (no markdown fences, no commentary):
{
  "rubric": {
    "A": {"content": {"correctness": 1-5, "completeness": 1-5, "accuracy": 1-5},
          "structure": {"organization": 1-5, "formatting": 1-5, "usability": 1-5}},
    "B": {"content": {...}, "structure": {...}}
  },
  "output_quality": {
    "A": {"score": 1-10, "strengths": ["..."], "weaknesses": ["..."]},
    "B": {"score": 1-10, "strengths": ["..."], "weaknesses": ["..."]}
  },
  "winner": "A"|"B"|"tie",
  "reasoning": "brief explanation of the verdict"
}"""


def load_output(run_dir: Path) -> str:
    outputs_dir = run_dir / "outputs"
    response = (
        (outputs_dir / "response.txt").read_text(encoding="utf-8", errors="replace")
        if (outputs_dir / "response.txt").exists()
        else ""
    )
    files_dir = outputs_dir / "files"
    blocks = [f"FINAL RESPONSE TEXT:\n{response}"]
    if files_dir.exists():
        for path in sorted(files_dir.rglob("*")):
            if path.is_file():
                content = path.read_text(encoding="utf-8", errors="replace")[:4000]
                blocks.append(f"--- FILE {path.relative_to(files_dir)} ---\n{content}")
    return "\n\n".join(blocks)


def average(scores: dict) -> float:
    values = [v for group in scores.values() for v in group.values()]
    return round(sum(values) / len(values), 2) if values else 0.0


def compare_eval(eval_meta: dict, with_dir: Path, without_dir: Path) -> dict:
    output_with = load_output(with_dir)
    output_without = load_output(without_dir)

    # Blind, random A/B assignment so the judge can't infer which is which.
    with_is_a = random.random() < 0.5
    output_a, output_b = (
        (output_with, output_without) if with_is_a else (output_without, output_with)
    )
    label_map = {
        "A": "with_skill" if with_is_a else "without_skill",
        "B": "without_skill" if with_is_a else "with_skill",
    }

    user_prompt = f"""TASK PROMPT:
{eval_meta["prompt"]}

EXPECTED OUTPUT:
{eval_meta.get("expected_output", "(none given)")}

OUTPUT A:
{output_a[:6000]}

OUTPUT B:
{output_b[:6000]}
"""
    result = chat_json(
        [
            {"role": "system", "content": COMPARATOR_SYSTEM_PROMPT},
            {"role": "user", "content": user_prompt},
        ],
        model=JUDGE_MODEL,
        max_tokens=3500,
    )

    rubric = result.get("rubric", {})
    for label in ("A", "B"):
        r = rubric.get(label, {})
        r["content_score"] = average({"content": r.get("content", {})})
        r["structure_score"] = average({"structure": r.get("structure", {})})
        r["overall_score"] = round(
            (r["content_score"] + r["structure_score"]) / 2 * 2, 2
        )  # scale ~1-5 -> ~1-10

    winner_label = result.get("winner", "tie")
    winner_config = label_map.get(winner_label, "tie") if winner_label in ("A", "B") else "tie"

    return {
        "label_map": label_map,
        "winner_label": winner_label,
        "winner_config": winner_config,
        "reasoning": result.get("reasoning", ""),
        "rubric": rubric,
        "output_quality": result.get("output_quality", {}),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description="Layer 2 blind comparator (agent-rubric)")
    parser.add_argument("--workspace", required=True)
    parser.add_argument("--iteration", type=int, default=1)
    parser.add_argument("--only", default=None)
    parser.add_argument("--force", action="store_true")
    args = parser.parse_args()

    iteration_dir = Path(args.workspace).resolve() / f"iteration-{args.iteration}"
    only = set(args.only.split(",")) if args.only else None

    wins = {"with_skill": 0, "without_skill": 0, "tie": 0}
    for eval_dir in sorted(iteration_dir.glob("eval-*")):
        meta_path = eval_dir / "eval_metadata.json"
        if not meta_path.exists():
            continue
        eval_meta = json.loads(meta_path.read_text())
        if only and eval_meta["eval_name"] not in only:
            continue

        with_dir = eval_dir / "with_skill" / "run-1"
        without_dir = eval_dir / "without_skill" / "run-1"
        if not (with_dir.exists() and without_dir.exists()):
            continue
        comparison_path = eval_dir / "comparison.json"
        if comparison_path.exists() and not args.force:
            print(
                f"[comparator] {eval_meta['eval_name']}: skip (comparison.json exists)",
                file=sys.stderr,
            )
            comparison = json.loads(comparison_path.read_text())
            wins[comparison["winner_config"]] += 1
            continue

        print(f"[comparator] {eval_meta['eval_name']} ...", file=sys.stderr, flush=True)
        comparison = compare_eval(eval_meta, with_dir, without_dir)
        (eval_dir / "comparison.json").write_text(json.dumps(comparison, indent=2))
        wins[comparison["winner_config"]] += 1
        print(
            f"[comparator] {eval_meta['eval_name']}: winner={comparison['winner_config']} "
            f"({comparison['reasoning'][:100]})",
            file=sys.stderr,
        )

    print(f"[comparator] wins: {wins}", file=sys.stderr)


if __name__ == "__main__":
    main()
