#!/usr/bin/env python3
"""Layer 3 grader: evaluate each run's expectations against its outputs.

Adapted from vendor/skill-creator/agents/grader.md (Anthropic, Apache-2.0),
but judged by an LLM call (glm-5.2 via NVIDIA NIM) instead of a Claude Code
subagent, and produces grading.json in the exact schema from
vendor/skill-creator/references/schemas.md so the vendored
aggregate_benchmark.py can consume it unmodified.

Usage:
    python3 grader.py --workspace ./workspace/journalist --iteration 1
"""

import argparse
import json
import sys
from pathlib import Path

from nim_client import chat_json

MAX_FILE_CHARS = 4000
JUDGE_MODEL = "z-ai/glm-5.2"

GRADER_SYSTEM_PROMPT = """You are the Grader for an agent-skill evaluation, adapted from Anthropic's \
skill-creator grader agent. You review what an AI agent actually did (its final \
response, the files it created/modified, and a summary of its tool calls) and \
determine whether each expectation passes or fails, with concrete evidence.

Grading criteria:
- PASS only when there is clear, specific evidence the expectation is true, and \
that evidence reflects genuine task completion (not superficial compliance, e.g. \
a file existing with the right name but wrong/empty content).
- FAIL when there is no evidence, the evidence contradicts the expectation, or the \
evidence is superficial/coincidental.
- The burden of proof to PASS is on the expectation. When uncertain, FAIL.
- Be specific: evidence must quote or precisely describe what you found.

Respond with ONLY a JSON object (no markdown fences, no commentary) of the form:
{
  "expectations": [
    {"text": "<the expectation text, verbatim>", "passed": true|false, "evidence": "<specific evidence>"}
  ]
}
The expectations array must have exactly one entry per expectation given, in the \
same order, using the exact same text."""


def truncate(text: str, limit: int = MAX_FILE_CHARS) -> str:
    if len(text) <= limit:
        return text
    return text[:limit] + f"\n... [truncated, {len(text) - limit} more chars]"


def build_user_prompt(
    eval_meta: dict, response_text: str, files: dict[str, str], metrics: dict
) -> str:
    files_block = "\n\n".join(
        f"--- {rel} ---\n{truncate(content)}" for rel, content in files.items()
    )
    if not files_block:
        files_block = "(no files were created or modified)"

    expectations = eval_meta.get("assertions", [])
    expectations_block = "\n".join(f"{i + 1}. {text}" for i, text in enumerate(expectations))

    return f"""EVAL PROMPT GIVEN TO THE AGENT:
{eval_meta["prompt"]}

EXPECTED OUTPUT (human description of success):
{eval_meta.get("expected_output", "(none given)")}

AGENT'S FINAL RESPONSE TEXT:
{truncate(response_text, 3000) or "(empty)"}

FILES CREATED OR MODIFIED BY THE AGENT:
{files_block}

TOOL CALL SUMMARY:
{json.dumps(metrics.get("tool_calls", {}))} (total_tool_calls={metrics.get("total_tool_calls", 0)}, \
errors_encountered={metrics.get("errors_encountered", 0)})

EXPECTATIONS TO GRADE:
{expectations_block}
"""


def load_files(outputs_dir: Path) -> dict[str, str]:
    files_dir = outputs_dir / "files"
    if not files_dir.exists():
        return {}
    out = {}
    for path in sorted(files_dir.rglob("*")):
        if path.is_file():
            try:
                out[str(path.relative_to(files_dir))] = path.read_text(
                    encoding="utf-8", errors="replace"
                )
            except OSError:
                pass
    return out


def grade_run(eval_meta: dict, run_dir: Path) -> dict:
    outputs_dir = run_dir / "outputs"
    response_text = (
        (outputs_dir / "response.txt").read_text(encoding="utf-8", errors="replace")
        if (outputs_dir / "response.txt").exists()
        else ""
    )
    metrics = (
        json.loads((outputs_dir / "metrics.json").read_text())
        if (outputs_dir / "metrics.json").exists()
        else {}
    )
    timing = (
        json.loads((run_dir / "timing.json").read_text())
        if (run_dir / "timing.json").exists()
        else {}
    )
    files = load_files(outputs_dir)

    expectations = eval_meta.get("assertions", [])
    if not expectations:
        graded = []
    else:
        prompt = build_user_prompt(eval_meta, response_text, files, metrics)
        result = chat_json(
            [
                {"role": "system", "content": GRADER_SYSTEM_PROMPT},
                {"role": "user", "content": prompt},
            ],
            model=JUDGE_MODEL,
            max_tokens=4000,
        )
        graded = result.get("expectations", [])
        # Defensive: make sure we have exactly one graded entry per expectation,
        # falling back to a FAIL if the judge dropped one.
        if len(graded) != len(expectations):
            by_text = {g.get("text", ""): g for g in graded}
            graded = [
                by_text.get(
                    text,
                    {
                        "text": text,
                        "passed": False,
                        "evidence": "Judge did not grade this expectation.",
                    },
                )
                for text in expectations
            ]

    passed = sum(1 for g in graded if g.get("passed"))
    total = len(graded)
    grading = {
        "expectations": graded,
        "summary": {
            "passed": passed,
            "failed": total - passed,
            "total": total,
            "pass_rate": round(passed / total, 4) if total else 0.0,
        },
        "execution_metrics": {
            "tool_calls": metrics.get("tool_calls", {}),
            "total_tool_calls": metrics.get("total_tool_calls", 0),
            "total_steps": metrics.get("total_steps", 0),
            "errors_encountered": metrics.get("errors_encountered", 0),
            "output_chars": metrics.get("output_chars", 0),
            "transcript_chars": metrics.get("transcript_chars", 0),
        },
        "timing": {
            "total_duration_seconds": timing.get("total_duration_seconds", 0.0),
        },
    }
    return grading


def main() -> None:
    parser = argparse.ArgumentParser(description="Layer 3 grader")
    parser.add_argument("--workspace", required=True)
    parser.add_argument("--iteration", type=int, default=1)
    parser.add_argument("--only", default=None, help="Comma-separated eval names to grade")
    parser.add_argument(
        "--force", action="store_true", help="Re-grade even if grading.json already exists"
    )
    args = parser.parse_args()

    iteration_dir = Path(args.workspace).resolve() / f"iteration-{args.iteration}"
    only = set(args.only.split(",")) if args.only else None

    for eval_dir in sorted(iteration_dir.glob("eval-*")):
        meta_path = eval_dir / "eval_metadata.json"
        if not meta_path.exists():
            continue
        eval_meta = json.loads(meta_path.read_text())
        if only and eval_meta["eval_name"] not in only:
            continue

        for mode in ("with_skill", "without_skill"):
            run_dir = eval_dir / mode / "run-1"
            if not run_dir.exists():
                continue
            grading_path = run_dir / "grading.json"
            if grading_path.exists() and not args.force:
                print(
                    f"[grader] {eval_meta['eval_name']} / {mode}: skip (grading.json exists)",
                    file=sys.stderr,
                )
                continue
            print(f"[grader] {eval_meta['eval_name']} / {mode} ...", file=sys.stderr, flush=True)
            grading = grade_run(eval_meta, run_dir)
            grading_path.write_text(json.dumps(grading, indent=2))
            summary = grading["summary"]
            print(
                f"[grader] {eval_meta['eval_name']} / {mode}: {summary['passed']}/{summary['total']} passed",
                file=sys.stderr,
            )


if __name__ == "__main__":
    main()
