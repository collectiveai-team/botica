# Vendored from anthropics/skills

The files in this directory are vendored, unmodified except where noted,
from Anthropic's reference `skill-creator` skill, which implements the
[agentskills.io evaluation standard](https://agentskills.io/skill-creation/evaluating-skills)
(evals.json / grading.json / timing.json / benchmark.json schemas).

- Source: https://github.com/anthropics/skills/tree/main/skills/skill-creator
- Commit: `b9e19e6f44773509fbdd7001d77ff41a49a486c1` (2026-04-20)
- License: Apache License 2.0 (see `LICENSE.txt` in this directory)

Vendored files:
- `scripts/aggregate_benchmark.py`, `scripts/utils.py` — used as-is to roll
  per-run `grading.json`/`timing.json` files into `benchmark.json`/`benchmark.md`.
- `agents/grader.md`, `agents/comparator.md`, `agents/analyzer.md` — reference
  prompts for grading, blind comparison, and benchmark-pattern analysis. Our
  `grader.py`/`comparator.py` adapt these prompts for a non-Claude-Code judge
  model (glm-5.2 via the NVIDIA NIM OpenAI-compatible endpoint) instead of a
  Claude Code subagent.
- `references/schemas.md` — canonical JSON schemas this pipeline targets.

Everything else under `tools/skill-eval/` (the executor, grader, comparator,
and orchestration) is original to this repo and drives OpenCode directly
(via Promptfoo's `opencode:sdk` provider) rather than Claude Code, since this
repo's target agent runtime is OpenCode.
