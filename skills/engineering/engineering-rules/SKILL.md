---
name: engineering-rules
description: Look up what the house engineering ruleset says about code that is being written, reviewed, refactored or restructured, such as which linter, which package layout, which test shape, or whether a pattern is current or legacy. It holds the rules that no tool measures, for Python and its common stacks. Invoked by engineering-pr-review and engineering-refactor, and usable directly.
---

# Engineering Rules

The single source of the house engineering rules that no tool measures. This skill owns that
ruleset; the workflows that apply it (`engineering-pr-review`, `engineering-refactor`) invoke
this one rather than carrying their own copy.

The rules that `botica lint` measures deterministically are **not here**: the catalog of botica
is their only documentation. `uvx botica rules` lists them and `uvx botica explain <slug>`
describes one. A rule id that this bundle cites and that has no file in `references/` is one of them.

## Reading protocol

Read by need, not all at once. Loading the whole ruleset wastes context that the actual work needs.

1. **`references/_sections.md`** — the index. Every rule id, grouped by section, one line each.
   Start here always.
2. **`references/<id>.md`** — the specific rules the work touches. Read only these.

Rule ids are kebab-case and section-prefixed (`py-package-manager` is in section `py`), so the
index tells you which files to open from the file paths alone.

## Code that a rule points at

A rule can cite topics: code and documentation to read, kept in `references/` of the project or
built into botica. Before you write code that a rule covers, run
`uvx botica refs show <rule id, CES-number or topic>` and follow the pattern it prints. A rule that
has a topic says so in its page on the site; `uvx botica refs list` lists every topic and who cites
it. In a project with no `botica`, skip this step and use the rule text alone.

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
`references/py-no-formatter-churn.md`". The id is the stable name; the path is an implementation
detail of this bundle.

## Relationship to the CES catalog

In a repo scaffolded by `collectiveai-team/botica`, the house standards that botica does not
measure also ship as always-on context: an index in `AGENTS.md`, each entry with a `CES-<number>`
code. The text of each standard is `standards/<set>/<number>-<slug>.md` in the botica repository
and on its site (see `docs/engineering-standards.md`). This bundle carries the rules into repos
that have no such index.
