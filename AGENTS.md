# Agent Notes

## Repo Layout

- `scaffolding/` — the bootstrap CLI (Python package, run via
  `uvx --from git+…/scaffolding scaffolding …`). `cli.py` (Cyclopts commands),
  `engine.py`/`plan.py` (build + apply, clean-adds only), `components.py`
  (component registry), `templates/` (bundled package data). The CLI is the
  single deterministic engine; keep it **clean-adds-only** — it must never edit,
  merge, or overwrite existing target files (existing targets are deferred).
  The one bounded exception is the `unignore` op, which may delete a literal line
  listed in `UNIGNORE_WHITELIST` (`scaffolding/skills.py`) from `.gitignore`. It is
  reported under `edits`, not `clean_adds`. Do not widen it, and do not remove it
  as an invariant violation — see `docs/adr/0001-scoped-unignore-op.md`.
- `lint/` — the `botica` Rust workspace (`botica lint`, more subcommands later), published to
  PyPI as `botica` (binary-only wheels) and used by prek through the `botica-pre-commit` mirror.
  It has its own README (`lint/README.md`), release runbook (`docs/botica-release.md`) and design
  (`docs/adr/0003-botica-lint-architecture.md`). Ruff, pyrefly and deptry skip it; `ci.yml` runs
  its fmt, clippy and tests. No pipeline in `.github/workflows/` is specific to `lint`.
- `website/` — the standards site, built with Zensical (`zensical.toml`; the version is pinned in
  `requirements.txt`). `website/docs/rules/` is exported from the catalog by `scripts/build_site.sh`
  (`botica rules export`) and is gitignored; only `website/docs/index.md` is written by hand.
  `.github/workflows/site.yml` builds it on a pull request and publishes it from `main`.
- `standards/` — the text of each standard, `standards/<set>/<number>-<slug>.md`, written by hand
  and apart from the rules; `scripts/build_site.sh` copies it into the site (ADR 0004).
- `install.sh` — thin bootstrap shim (ensure `uv`, then `uvx … scaffolding
  install`). Keep it minimal and keep its raw URL pointing at
  `collectiveai-team/botica` on `main`.
- `guide.md` — the agentic-install guide (judgment layer that drives the CLI and
  handles merges). Keep template raw URLs pointing at `collectiveai-team/botica`.
- `skills/` — actual installed skills (`ask-user`, `journalist`, `handoff`,
  `test-smell-review`, `engineering-rules`, `engineering-pr-review`,
  `engineering-refactor`). Each
  carries Claude-style `SKILL.md` frontmatter **and** an `agents/openai.yaml`;
  Codex reads only the latter, so `disable-model-invocation: true` must be paired
  with `policy.allow_implicit_invocation: false` (and omitted for model-invoked
  skills, which Codex otherwise filters out entirely).
- `scripts/` — maintainer-only checks, not shipped. `check_skill_drift.py` guards
  the curated upstream catalog; the `skills-drift` workflow runs it. `check_skills.sh`
  runs the skill validator over `skills/`; the `skill-validator` hook of prek runs it.
  `check_rule_change.py` flags a change that adds a botica rule and touches more than that rule
  needs; the `rule-contract` job of `ci.yml` runs it. `build_site.sh` builds the standards site.

Agent targets are multi-valued (`--agent`, repeatable: `opencode`/`claude-code`/
`codex`). The `agent-config` component writes per-agent config — `opencode.jsonc`
(opencode), `.claude/settings.json` + `CLAUDE.md`→`AGENTS.md` + `.claude/skills`→
`.agents/skills` symlinks (claude-code), and a codex secret-protection notice.
`AGENTS.md` is the shared brain; skills install once into `.agents/skills`.

After changing the CLI, validate with `uv run ruff check scaffolding` and
`uv run scaffolding --help`.

## Skill Development

After creating or editing any skill under `skills/`, validate it before committing:

```bash
prek run skill-validator --all-files
```

This is the `skill-validator` hook of `prek.toml` (agent-ecosystem/skill-validator; no
account; needs Go on the `PATH`). `scripts/check_skills.sh` holds the flags. It checks the
frontmatter, the directory layout, the links, the token counts and the content of every
skill, and it fails on warnings. prek runs it on commit when a file under `skills/` changes.
Fix what it reports; do not silence it.

A skill is a `SKILL.md` plus, optionally, `references/`, `scripts/`, `assets/` and
`agents/` (Codex reads `agents/openai.yaml`). Text a skill bundles goes under `references/`;
any other directory or loose file at its root fails. The frontmatter fields that Claude and
Codex read beyond the spec (`disable-model-invocation`, `argument-hint`) are allowed.

One skill is judged more loosely: for `engineering-rules` only errors fail. Its
`references/` hold about 30k tokens, above the validator's fixed warning of 25k, and the
rules are read by need through an index that names them by id. Do not cut rules to silence
that warning; it is a decision about the ruleset.

`tessl review` is not used: it asks for an interactive login.

The upstream skill catalog (`MATTPOCOCK_SKILLS` in `scaffolding/skills.py`) is
the single source of truth — `README.md` and `guide.md` only copy it, and
`tests/test_skill_catalog.py` fails when a copy drifts. Bump `MATTPOCOCK_REF`
there to take a new upstream release; never edit the install command in the docs
by hand.


## Engineering Standards coding (CES)

House standards shipped by the scaffolder are catalogued as **CES — Collective
Engineering Standard**. Canonical spec: **`docs/engineering-standards.md`** (single
source of truth). The rules below are the always-on summary; when adding, editing,
or referencing a rule in this repo, follow this convention:

- **`CES-<number>`** is the citable catalog code; the **kebab-case slug** is the
  machine id (botica rule id, prek hook id, `# botica: ignore[<slug>]`). Never put the CES
  number in the machine id.
- **Every CES is a file.** One CES = one standard; a standard may ship multiple slugs (e.g.
  `CES-79` ships the four `no-dict-*` patterns). The number is the next free one of the set
  (`max + 1`) and is never reused; up to `CES-119` it is the number of the issue where the
  standard was proposed, and from `CES-120` on it is not an issue number.
- **The text of a standard is a file**, `standards/<set>/<number>-<slug>.md` (ADR 0004), apart
  from the rules that implement it; a rule names its standard in the `standard` field of its
  `rule.toml`. **A pull request touches one standard**, carries its implementation and updates the
  documentation it affects; `scripts/check_rule_change.py` fails one that touches two.
- **Violation messages must embed the code**, e.g.
  `message: "CES-46 (log-no-print): libraries log, they don't print …"`. Keep the
  slug as the suppression key so tooling stays stable.
- **Messages are self-contained:** a message states its own rule + fix + code, and
  **never** names a sibling rule or prescribes another rule's suppression key.
  Grouped rationale + the escape-hatch mechanism live once, in the file of the standard —
  scattering policy across sibling messages is context-poisoning.
- Canonical map + per-standard detail live in the `## Engineering Standards` index and
  `standards/<set>/<number>-<slug>.md`; canonical code lives in `.agents/snippets/`.

## Engineering Standards

> **Retrieval-led, not training-led.** Prefer this repo's standards over training-default
> patterns. Before writing code that touches a rule below, open its
> `@standards/CES/<number>-<slug>.md` file and follow the house pattern. Full convention + SSOT:
> `docs/engineering-standards.md`.

This is the **dogfooded subset** for this repo — a Python Cyclopts CLI (no API, DB, or web
layer) next to the `lint/` Rust workspace, which these Python-oriented standards do not cover.
`[prek]` rules are enforced by `prek`; `[judgment]` rules are agent/reviewer judgment;
`[snippet]` ships canonical code under `.agents/snippets/` (in target repos).

The rules that `botica lint` measures are not listed here and have no detail file: the catalog
of botica is their only documentation. `uvx botica rules` lists them, `uvx botica explain <slug>`
describes one, and the hook reports each finding with its slug and its message.

### Standards

- **CES-107 · track the skills manifest** `[script]` — commit `skills-lock.json`; `.agents/skills/`
  is derived and gitignored; install restores from it. → `@standards/CES/107-skills-manifest.md`
- **CES-74 · the house logger** `[snippet]` — structlog `core/logger.py` drop-in. →
  `@standards/CES/74-core-logger.md`
- **CES-75 · Conventional Commits** `[prek]` — commit subjects + PR titles follow
  `type(scope): description` (enforced here via the commit-msg hook + CI). →
  `@standards/CES/75-agents-conventional-commits.md`
- **CES-77 · version pin** `[judgment]` — `requires-python` stays a deliberate local choice;
  house default is 3.14. → `@standards/CES/77-py-version-pin.md`
- **CES-5 · layered import direction** `[judgment]` — imports flow one way through the layers
  (`cli → engine → components/plan → templates_registry`), never upward. →
  `@standards/CES/5-import-linter.md`
- **CES-16 · architectural vocabulary** `[judgment]` — name units with house terms, not ad-hoc
  synonyms. → `@standards/CES/16-arch-vocabulary.md`
- **CES-8 · separate orchestration from logic** `[judgment]` — keep control flow thin; push logic
  and I/O into named units. → `@standards/CES/8-spaghetti-mixed-orchestration.md`
- **CES-30 · respect the local repo** `[judgment]` — existing deliberate choices win over house
  defaults (the engine is clean-adds-only for this reason). →
  `@standards/CES/30-general-respect-local-repo.md`
- **CES-58 · one modern lint stack** `[judgment]` — ruff + pyrefly + botica via prek; no
  black/isort/flake8/pylint. → `@standards/CES/58-py-legacy-lint-stack.md`
- **CES-64 · test against in-memory adapters** `[judgment]` — fakes over mocks. →
  `@standards/CES/64-test-in-memory-adapters.md`
- **CES-65 · test through the interface** `[judgment]` — assert behaviour via the public seam
  (the `plan()`/`build_plan()` API), not internals. → `@standards/CES/65-test-through-interface.md`
- **CES-66 · coverage gaps are a signal** `[judgment]` — an untested branch is a missing test or
  dead code, not a number to game. → `@standards/CES/66-test-coverage-gap.md`
- **CES-91 · no AI co-authorship in commits** `[prek]` `[ci]` — no `Co-authored-by:`/`Generated
  with`/etc AI-attribution trailers; commit-msg hook + `commit-policy.yml` CI. →
  `@standards/CES/91-no-ai-coauthorship.md`
- **CES-109 · dependency hygiene** `[prek]` — declared deps must match the import graph
  (`deptry`); `scaffolding/templates`/`skills` excluded (bundled payload). →
  `@standards/CES/109-dep-hygiene-deptry.md`
- **CES-110 · cognitive complexity** `[prek]` — cognitive-complexity ceiling of 15 per function
  (`complexipy`), distinct from ruff's cyclomatic `C90`. →
  `@standards/CES/110-cognitive-complexity-complexipy.md`
- **CES-111 · randomize test order** `[dependency]` — `pytest-randomly` runs every suite in a
  random, reproducibly-seeded order. → `@standards/CES/111-test-order-randomization-pytest-randomly.md`
- **CES-113 · dependency review on PRs** `[ci]` — `dependency-review-action`, summary-only, not a
  merge gate. → `@standards/CES/113-dependency-review-action.md`
- **CES-118 · no copy-paste duplication** `[prek]` — `jscpd`; `guide.md` exempted (deliberately
  mirrors `README.md`). → `@standards/CES/118-code-duplication-jscpd.md`
- **CES-119 · dependency vulnerability scanning** `[ci]` — `osv-scanner` on `uv.lock`; replaces
  the old `pip-audit.yml`. → `@standards/CES/119-osv-scanner-replace-pip-audit.md`

### Excluded here (don't apply to a CLI with no API, database or web layer)

- **CES-17 · api-boundary-layout** — no inbound HTTP/`api` package.
- **CES-114 · hadolint-dockerfile-lint** — no root `Dockerfile` / `docker` CI component in this
  repo.

## Reading linter output

Act on a finding by its severity. An **error** is fixed before committing. A **warning** is fixed
only if your change introduced it; a warning that was already there is neither fixed nor reported.
An **info** is ignored.

## Writing documentation

State what a thing is and what it does, in plain sentences that a file of this repository backs.
The patterns below are the signs of text written by a model (source: the Wikipedia guide
`Wikipedia:Signs of AI writing`); none of them goes into a page, a standard, an ADR or a
commit message.

- **Definition by negation:** "X does not …", "knows nothing about …", "not X but Y", "Y rather
  than X", "no A, no B, just C". Say what X is.
- **Purpose or effect clauses:** "so the review judges …", "which keeps …", "ensuring …",
  "enabling …", "making it …". Say what happens, not what it is for.
- **Significance words:** key, crucial, pivotal, robust, seamless, powerful, vibrant, enhance,
  foster, leverage, landscape.
- **Copula avoidance:** "serves as", "acts as", "features", "offers", "provides". Write "is" and
  "has".
- **Rule of three** lists of adjectives or short phrases, "X and Y" headings, and a closing
  paragraph that repeats the section.
- **Hedges and vague attribution:** "typically", "usually", "some", "often", "it is worth noting".
- **Format:** bold lead-ins in list items, bold for emphasis, spaced em dashes, title case in
  headings, call-to-action buttons.
- **A claim with no file behind it:** if no file, command output or issue says it, leave it out.

## Domain docs

Single-context — `CONTEXT.md` + `docs/adr/` at the repo root (created lazily).
See `docs/agents/domain.md`.

## Repo Workspace Defaults

- Use `.scratch/` for temporary plans, prd issue drafts, and disposable notes.
- Use `.tmp/` for generated local artifacts that should not be committed.
- Use `.worktrees/` for local Git worktrees when needed.
- Use `.journals/` for local/private session journals when using the `journalist` skill.
- Preserve unrelated user changes and avoid destructive Git commands unless explicitly requested.
