---
name: engineering-rules
description: The house engineering ruleset — Python toolchain, package layout, deep-module architecture, testing, logging, and the stack-specific patterns (FastAPI, Prefect, LLM/AI, ASR, Next.js, k8s). Use when you need to look up what the house standard says about code you are writing, reviewing, or refactoring. Invoked by engineering-pr-review and engineering-refactor, and usable directly.
---

# Engineering Rules

The single source of the house engineering rules. This skill owns the ruleset; the workflows
that apply it (`engineering-pr-review`, `engineering-refactor`) invoke this one rather than
carrying their own copy.

## Reading protocol

Read by need, not all at once. The ruleset is ~3,800 lines; loading it whole wastes context
that the actual work needs.

1. **`rules/_sections.md`** — the index. Every rule id, grouped by section, one line each.
   Start here always.
2. **`rules/<id>.md`** — the specific rules the work touches. Read only these.
3. **`AGENTS.md`** — the whole ruleset compiled into one document. For skimming everything at
   once; not the default path.

Rule ids are kebab-case and section-prefixed (`py-package-manager` is in section `py`), so the
index tells you which files to open from the file paths alone.

## Judging a rule before applying it

Each rule carries two fields that decide whether it applies here at all:

- **`applies-to`** — `all`, or a stack tag (`python`, `fastapi`, `prefect`, `asr`, `llm`,
  `nextjs`, `k8s`, `edge`). A Prefect rule says nothing about a repo with no Prefect.
- **`status`** — `current` (what the repos do today), `direction` (where new or modernized
  code should head), `legacy` (preserve where it exists; don't expand it, don't fight it).

**The local repo wins over the direction** unless the user explicitly asked to modernize
(`general-respect-local-repo`). A repo's package manager, lockfile, CI, configured linter, and
public interfaces are contracts — judge code against what its repo actually uses, not against
a one-size-fits-all standard.

## Citing rules

Cite the **rule id**, not a file path: "violates `py-no-formatter-churn`", not "violates
`rules/py-no-formatter-churn.md`". The id is the stable name; the path is an implementation
detail of this bundle.

## Relationship to the CES catalog

In a repo scaffolded by `collectiveai-team/scaffolding`, the house standards also ship as
always-on context: an index in `AGENTS.md` plus detail files under `.agents/rules/<slug>.md`,
each carrying a `CES-<issue#>` code (see `docs/engineering-standards.md`).

**Where both are present, `.agents/rules/` is authoritative** — it is the tracked, coded,
enforced catalog, and several rule ids here share its slugs. This bundle exists to carry the
rules into repos that have no `.agents/rules/` at all.
