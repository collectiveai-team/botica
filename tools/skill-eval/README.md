# skill-eval — measuring in-house agent skills

A 3-layer pipeline for measuring whether a skill under `skills/` actually
helps an agent, instead of just vibes-checking it.

| Layer | What it measures | Tool |
|---|---|---|
| **1 — structural** | Is `SKILL.md` well-formed (frontmatter, spec compliance)? | [`agnix`](https://github.com/agent-sh/agnix) (static lint, no LLM calls) |
| **2 — agent-rubric** | Holistic output quality (organization, formatting, usability), blind A/B | `comparator.py` |
| **3 — behavioral/outcome** | Does the skill change what a real agent *does*, per explicit pass/fail expectations? | `executor.py` + `grader.py` |

Layers 2 and 3 both compare **with the skill loaded** vs **without it**, for
the same prompts, so the delta is attributable to the skill rather than to
random model variance.

## Why this shape, not something ad hoc

`agentskills.io` (the org behind the open SKILL.md format) publishes a
concrete [evaluation standard](https://agentskills.io/skill-creation/evaluating-skills):
an `evals/evals.json` file per skill, `with_skill`/`without_skill` run
directories, `grading.json`/`timing.json` per run, and an aggregated
`benchmark.json`/`benchmark.md`. Anthropic's own `skill-creator` skill
implements it. We follow that schema (see `vendor/skill-creator/references/schemas.md`)
instead of inventing our own, and vendor the reference `aggregate_benchmark.py`
(Apache-2.0, unmodified — see `vendor/skill-creator/NOTICE.md`) rather than
re-deriving the same mean/stddev/delta rollup.

The standard itself expects **real agent executions**, not chat completions:
> "For each run, provide: the skill path, the test prompt, input files, the
> output directory... In environments that support subagents (Claude Code,
> for example), this isolation comes naturally... Without subagents, use a
> separate session for each run."

`skill-creator` satisfies that by spawning Claude Code subagents. This repo's
target agent runtime is **OpenCode**, not Claude Code, so `executor.py` drives
a real OpenCode agent directly instead (see "Driver" below).

## Driver: why not Promptfoo, why not a chat-completion harness

Two things were tried and rejected before landing on the current driver:

1. **A pure chat-completion harness** (e.g. `agent-skills-eval` against a raw
   model API) cannot test skills that are inherently agentic — `journalist`
   tells the model to *read a file, write a file, run a script*. A model with
   no tools either stalls on those instructions or ignores them. That's a
   harness/skill mismatch, not a skill defect, and it understates every
   tool-driving skill's real value.
2. **Promptfoo's `opencode:sdk` provider** does drive a real OpenCode agent
   with tools, and got close — but `session.prompt()`'s return value there
   only carries the *last* message's parts. For a multi-step tool-calling
   turn (exactly what `journalist` needs: read index → decide topic → write
   file → run script), that means the tool-call trace/transcript it exposes
   is incomplete, which breaks `metrics.json`'s `tool_calls` breakdown and
   `files_created` detection.

`oc_driver.mjs` talks to the same `@opencode-ai/sdk` (v2) directly: it calls
`session.prompt()` (which blocks until the whole multi-step turn finishes)
and then `session.messages()` to fetch **every** message/part generated
during that turn, giving `executor.py` the full tool-call trace. It also
disables undici's default 300s `headersTimeout` (session.prompt() genuinely
doesn't respond until the entire agentic turn is done, which routinely
exceeds 5 minutes for tool-heavy skills — without this the fetch fails with a
generic, undiagnosable "fetch failed").

## Models

- **Executor (target) model**: `anthropic/claude-sonnet-5`, via OpenCode's
  own stored Anthropic OAuth. We first tried `nvidia/z-ai/glm-5.2` (same
  model as the eval prompts were originally drafted against, hosted on
  NVIDIA NIM under a working API key) for consistency, but it needs 1–4
  minutes *per LLM call* on that endpoint — impractically slow for
  multi-step agentic turns that need several calls each. `claude-sonnet-5`
  is both faster per step and more reliable at tool use.
- **Judge model (grader.py / comparator.py)**: `z-ai/glm-5.2` via the NVIDIA
  NIM OpenAI-compatible endpoint (`nim_client.py`), read directly from
  OpenCode's own `auth.json` — never exported to the shell environment.
  Grading is a single fast text-in/JSON-out call, so NIM's latency isn't the
  bottleneck there the way it is for the agentic executor.

Swapping either model is a one-line change (`PROVIDER_ID`/`MODEL` in
`executor.py`, `JUDGE_MODEL` in `grader.py`/`comparator.py`).

## Usage

```bash
npm install   # promptfoo (kept for the vendored reference + future use),
              # agnix, @opencode-ai/sdk (installed as promptfoo's peer dep)

./run.sh ../../skills/productivity/journalist
# or step by step:
python3 executor.py   --skill ../../skills/productivity/journalist --workspace ./workspace/journalist --iteration 1
python3 grader.py      --workspace ./workspace/journalist --iteration 1
python3 vendor/skill-creator/scripts/aggregate_benchmark.py ./workspace/journalist/iteration-1 --skill-name journalist
python3 comparator.py  --workspace ./workspace/journalist --iteration 1
```

`executor.py --only <name1,name2>` and `grader.py`/`comparator.py --only ...`
let you re-run/re-grade a subset. `grader.py`/`comparator.py --force`
re-grades even if `grading.json`/`comparison.json` already exists.

## Adding evals for another skill

Add `evals/evals.json` to the skill directory (schema:
`vendor/skill-creator/references/schemas.md`). `files` entries are relative
to the skill directory and use the convention
`evals/files/<eval-name>/<path-to-place-in-the-fixture>`, e.g.
`evals/files/topic-reuse/.journals/index.md` gets copied to
`.journals/index.md` in that eval's isolated fixture directory.

## Results so far: `journalist`

`workspace/journalist/iteration-1/` (committed) has the full run. Headline:

| Metric | With skill | Without skill | Delta |
|---|---:|---:|---:|
| Pass rate | **100%** ± 0% | 65% ± 41% | **+35pp** |
| Time | 83.3s ± 37.6s | 144.1s ± 46.4s | −60.8s (skill is *faster*) |
| Tokens | 709 ± 59 | 706 ± 111 | +4 (negligible) |

Layer 2 (blind rubric): with_skill won 3/4 evals; without_skill won the
`redaction` eval (`claude-sonnet-5` redacts secrets by default regardless of
the skill, for this particular model — a legitimately different finding from
an earlier `glm-5.2` run, where the model parroted secrets back without the
skill's explicit rule).

Per-eval (Layer 3):

| Eval | With skill | Without skill | Note |
|---|:---:|:---:|---|
| `basic-entry` | 6/6 | 2/6 | without_skill uses `date`/`tags`/`commit` frontmatter, not the contract's `created_at`/`updated_at`/`title`/`topic`/`brief` |
| `topic-reuse` | 4/4 | 1/4 | without_skill invents a new topic instead of reusing `repo-tooling` from the provided index |
| `redaction` | 4/4 | 4/4 | both configs correctly omit the secrets (claude-sonnet-5 default behavior) |
| `update-in-place` | 5/5 | 5/5 | both configs correctly update in place given the existing entry as input — this eval doesn't discriminate for this model (see "Analyzing patterns" in the standard) |

The last two rows are an honest, useful finding, not noise: two of four evals
show real discriminating signal for `claude-sonnet-5` (format compliance,
topic reuse), and two don't (redaction, update-in-place) because this
particular model already does the right thing by default. The standard's own
guidance is to flag non-discriminating evals rather than launder them into
the headline number — which is exactly what the delta above already shows
(a genuine, if narrower, +35pp average across the whole set).
