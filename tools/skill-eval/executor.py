#!/usr/bin/env python3
"""Layer 3 executor: drive a skill's evals/evals.json through a real OpenCode
agent, with_skill and without_skill, and record results in the directory
layout + JSON schemas from the agentskills.io evaluation standard
(https://agentskills.io/skill-creation/evaluating-skills), as implemented by
Anthropic's skill-creator reference (see vendor/skill-creator/).

Unlike skill-creator (which spawns Claude Code subagents), this drives a
real OpenCode agent directly via `@opencode-ai/sdk` (see oc_driver.mjs),
since this repo's target agent runtime is OpenCode, not Claude Code. We
tried Promptfoo's `opencode:sdk` provider first, but `session.prompt()`'s
return value there only carries the last message's parts, which loses the
tool-call trace for multi-step turns -- oc_driver.mjs additionally calls
`session.messages()` to get the full turn.

Usage:
    python3 executor.py --skill ../../skills/productivity/journalist \
        --workspace ./workspace/journalist --iteration 1
"""

import argparse
import json
import shutil
import subprocess
import sys
import time
from pathlib import Path

TOOL_DIR = Path(__file__).resolve().parent
# Target model for the agentic (tool-using) executor runs. We tried
# nvidia/z-ai/glm-5.2 here first (same model as the other pipeline layers),
# but it needs 5-15 minutes per multi-step tool-calling turn on the NIM
# endpoint, which is impractically slow to iterate on. claude-sonnet-5 (via
# OpenCode's working Anthropic OAuth) is both faster per step and more
# reliable at tool use. The judge/grader stays on NIM glm-5.2 (see
# nim_client.py) since grading is a single fast turn, not the bottleneck.
PROVIDER_ID = "anthropic"
MODEL = "claude-sonnet-5"
DEFAULT_TIMEOUT_MS = 600_000


def slugify(text: str) -> str:
    return "".join(c if c.isalnum() else "-" for c in text.lower()).strip("-")


def load_evals(skill_dir: Path) -> dict:
    evals_path = skill_dir / "evals" / "evals.json"
    if not evals_path.exists():
        raise SystemExit(f"No evals/evals.json found under {skill_dir}")
    return json.loads(evals_path.read_text())


def snapshot_files(root: Path) -> dict[str, tuple[float, int]]:
    """Map relative path -> (mtime_ns, size), so we can detect both new AND
    in-place-modified files (an "update in place" eval, by design, doesn't
    create a new path -- it edits an existing one)."""
    if not root.exists():
        return {}
    out = {}
    for p in root.rglob("*"):
        if p.is_file():
            st = p.stat()
            out[str(p.relative_to(root))] = (st.st_mtime_ns, st.st_size)
    return out


def changed_files(
    before: dict[str, tuple[float, int]], after: dict[str, tuple[float, int]]
) -> list[str]:
    changed = [rel for rel, sig in after.items() if before.get(rel) != sig]
    return sorted(changed)


def prepare_fixture(skill_dir: Path, eval_item: dict, mode: str, tmp_root: Path) -> Path:
    """Create an isolated working directory for one run and seed input files."""
    fixture = tmp_root / f"fixture-{eval_item['name']}-{mode}"
    if fixture.exists():
        shutil.rmtree(fixture)
    fixture.mkdir(parents=True)

    prefix = f"evals/files/{eval_item['name']}/"
    for rel in eval_item.get("files", []):
        src = skill_dir / rel
        dest_rel = rel[len(prefix) :] if rel.startswith(prefix) else Path(rel).name
        dest = fixture / dest_rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dest)

    if mode == "with_skill":
        skills_dir = fixture / ".agents" / "skills"
        skills_dir.mkdir(parents=True, exist_ok=True)
        (skills_dir / skill_dir.name).symlink_to(skill_dir.resolve())

    return fixture


def build_tools(mode: str) -> dict:
    return {
        "read": True,
        "grep": True,
        "glob": True,
        "list": True,
        "write": True,
        "edit": True,
        "bash": True,
        "patch": False,
        "todowrite": False,
        "todoread": False,
        "webfetch": False,
        "question": False,
        "skill": mode == "with_skill",
        "lsp": False,
    }


def build_permission() -> dict:
    return {
        "skill": "allow",
        "bash": "allow",
        "edit": "allow",
        "write": "allow",
        "read": "allow",
        "grep": "allow",
        "glob": "allow",
        "list": "allow",
        "webfetch": "deny",
        "external_directory": "allow",
    }


def run_oc_driver(fixture: Path, prompt: str, mode: str, timeout_ms: int) -> dict:
    """Drive one turn through OpenCode directly (see oc_driver.mjs for why)."""
    payload = {
        "workingDir": str(fixture),
        "providerID": PROVIDER_ID,
        "modelID": MODEL,
        "prompt": prompt,
        "tools": build_tools(mode),
        "permission": build_permission(),
        "timeoutMs": timeout_ms,
    }
    proc = subprocess.run(
        ["node", str(TOOL_DIR / "oc_driver.mjs")],
        input=json.dumps(payload),
        cwd=TOOL_DIR,
        capture_output=True,
        text=True,
        timeout=(timeout_ms / 1000) + 60,
    )
    if proc.returncode != 0 and not proc.stdout.strip():
        return {"ok": False, "error": f"driver exited {proc.returncode}: {proc.stderr[-2000:]}"}
    try:
        return json.loads(proc.stdout.strip().splitlines()[-1])
    except (json.JSONDecodeError, IndexError):
        return {
            "ok": False,
            "error": f"unparseable driver output: {proc.stdout[-2000:]} {proc.stderr[-2000:]}",
        }


def extract_tool_metrics(parts: list[dict]) -> dict:
    tool_calls: dict[str, int] = {}
    errors = 0
    steps = 0
    for part in parts:
        ptype = part.get("type")
        if ptype == "step-start":
            steps += 1
        elif ptype == "tool":
            name = part.get("tool", "unknown")
            tool_calls[name] = tool_calls.get(name, 0) + 1
            if part.get("state", {}).get("status") == "error":
                errors += 1
    return {
        "tool_calls": tool_calls,
        "total_tool_calls": sum(tool_calls.values()),
        "total_steps": steps,
        "errors_encountered": errors,
    }


def run_one(skill_dir: Path, eval_item: dict, mode: str, run_dir: Path, tmp_root: Path) -> None:
    fixture = prepare_fixture(skill_dir, eval_item, mode, tmp_root)
    before = snapshot_files(fixture)

    run_dir.mkdir(parents=True, exist_ok=True)
    outputs_dir = run_dir / "outputs"
    outputs_dir.mkdir(parents=True, exist_ok=True)

    t0 = time.time()
    result = run_oc_driver(fixture, eval_item["prompt"], mode, DEFAULT_TIMEOUT_MS)
    wall_duration_ms = int((time.time() - t0) * 1000)

    (run_dir / "_driver_result.json").write_text(json.dumps(result, indent=2))

    if not result.get("ok"):
        (outputs_dir / "response.txt").write_text(
            f"[executor error] {result.get('error', 'unknown error')}"
        )
        metrics = {
            "tool_calls": {},
            "total_tool_calls": 0,
            "total_steps": 0,
            "files_created": [],
            "errors_encountered": 1,
            "output_chars": 0,
            "transcript_chars": 0,
        }
        timing = {
            "total_tokens": 0,
            "duration_ms": wall_duration_ms,
            "total_duration_seconds": wall_duration_ms / 1000,
        }
    else:
        output_text = result.get("output", "")
        token_usage = result.get("tokenUsage", {})
        parts = result.get("parts", [])

        (outputs_dir / "response.txt").write_text(output_text)
        (outputs_dir / "transcript.json").write_text(json.dumps(parts, indent=2))

        after = snapshot_files(fixture)
        created = changed_files(before, after)
        for rel in created:
            src = fixture / rel
            dest = outputs_dir / "files" / rel
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, dest)

        tool_metrics = extract_tool_metrics(parts)
        metrics = {
            **tool_metrics,
            "files_created": created,
            "output_chars": len(output_text),
            "transcript_chars": len(json.dumps(parts)),
        }
        duration_ms = result.get("durationMs", wall_duration_ms)
        timing = {
            "total_tokens": token_usage.get("total", 0),
            "duration_ms": duration_ms,
            "total_duration_seconds": round(duration_ms / 1000, 2),
        }

    (outputs_dir / "metrics.json").write_text(json.dumps(metrics, indent=2))
    (run_dir / "timing.json").write_text(json.dumps(timing, indent=2))

    fixture_archive = run_dir / "fixture"
    if fixture_archive.exists():
        shutil.rmtree(fixture_archive)
    shutil.move(str(fixture), str(fixture_archive))


def main() -> None:
    parser = argparse.ArgumentParser(description="Layer 3 executor (OpenCode-driven)")
    parser.add_argument(
        "--skill", required=True, help="Path to the skill directory (contains SKILL.md + evals/)"
    )
    parser.add_argument(
        "--workspace", required=True, help="Workspace root for results (sibling to the skill dir)"
    )
    parser.add_argument("--iteration", type=int, default=1)
    parser.add_argument(
        "--only", default=None, help="Comma-separated eval names to run (default: all)"
    )
    args = parser.parse_args()

    skill_dir = Path(args.skill).resolve()
    workspace = Path(args.workspace).resolve()
    iteration_dir = workspace / f"iteration-{args.iteration}"
    iteration_dir.mkdir(parents=True, exist_ok=True)

    evals_doc = load_evals(skill_dir)
    only = set(args.only.split(",")) if args.only else None

    tmp_root = iteration_dir / "_tmp"
    tmp_root.mkdir(exist_ok=True)

    for eval_item in evals_doc["evals"]:
        if only and eval_item["name"] not in only:
            continue
        eval_dir = iteration_dir / f"eval-{eval_item['id']}-{slugify(eval_item['name'])}"
        eval_dir.mkdir(parents=True, exist_ok=True)
        (eval_dir / "eval_metadata.json").write_text(
            json.dumps(
                {
                    "eval_id": eval_item["id"],
                    "eval_name": eval_item["name"],
                    "prompt": eval_item["prompt"],
                    "expected_output": eval_item.get("expected_output", ""),
                    "assertions": eval_item.get("expectations", []),
                },
                indent=2,
            )
        )

        for mode in ("with_skill", "without_skill"):
            run_dir = eval_dir / mode / "run-1"
            print(f"[executor] {eval_item['name']} / {mode} ...", file=sys.stderr, flush=True)
            t0 = time.time()
            run_one(skill_dir, eval_item, mode, run_dir, tmp_root)
            print(
                f"[executor] {eval_item['name']} / {mode} done in {time.time() - t0:.0f}s",
                file=sys.stderr,
                flush=True,
            )

    shutil.rmtree(tmp_root, ignore_errors=True)
    print(f"[executor] wrote results under {iteration_dir}", file=sys.stderr)


if __name__ == "__main__":
    main()
