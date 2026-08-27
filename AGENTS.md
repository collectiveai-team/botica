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
- `install.sh` — thin bootstrap shim (ensure `uv`, then `uvx … scaffolding
  install`). Keep it minimal and keep its raw URL pointing at
  `collectiveai-team/scaffolding` on `main`.
- `guide.md` — the agentic-install guide (judgment layer that drives the CLI and
  handles merges). Keep template raw URLs pointing at `collectiveai-team/scaffolding`.
- `skills/` — actual installed skills (`ask-user`, `journalist`, `handoff`). Each
  carries Claude-style `SKILL.md` frontmatter **and** an `agents/openai.yaml`;
  Codex reads only the latter, so `disable-model-invocation: true` must be paired
  with `policy.allow_implicit_invocation: false` (and omitted for model-invoked
  skills, which Codex otherwise filters out entirely).
- `scripts/` — maintainer-only checks, not shipped. `check_skill_drift.py` guards
  the curated upstream catalog; the `skills-drift` workflow runs it.

Agent targets are multi-valued (`--agent`, repeatable: `opencode`/`claude-code`/
`codex`). The `agent-config` component writes per-agent config — `opencode.jsonc`
(opencode), `.claude/settings.json` + `CLAUDE.md`→`AGENTS.md` + `.claude/skills`→
`.agents/skills` symlinks (claude-code), and a codex secret-protection notice.
`AGENTS.md` is the shared brain; skills install once into `.agents/skills`.

After changing the CLI, validate with `uv run ruff check scaffolding` and
`uv run scaffolding --help`.

## Skill Development

After creating or editing any skill under `skills/`, validate its `SKILL.md`
before committing:

```bash
tessl review run <SKILL.md>
```

Run it against each changed skill (e.g. `tessl review run skills/productivity/journalist/SKILL.md`)
and resolve the reported issues before publishing. It needs `tessl login`.

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

- **`CES-<issue#>`** is the citable catalog code; the **kebab-case slug** is the
  machine id (ast-grep `id:`, prek hook id, `.agents/rules/<slug>.md`,
  `# ast-grep-ignore: <slug>`). Never put the CES number in the machine id.
- **Every CES has a tracker issue.** One CES = one standard; a standard may ship
  multiple slugs (e.g. `CES-79` ships the four `no-dict-*` patterns). The issue id
  is the number — gaps from declined proposals are expected (PEP/RFC-style).
- **Already-implemented rules** get a retroactive AS-BUILT issue so they fit the
  invariant — never assign a code to a rule that only exists in code.
- **Violation messages must embed the code**, e.g.
  `message: "CES-46 (log-no-print): libraries log, they don't print …"`. Keep the
  slug as the suppression key so tooling stays stable.
- **Messages are self-contained:** a message states its own rule + fix + code, and
  **never** names a sibling rule or prescribes another rule's suppression key.
  Grouped rationale + the escape-hatch mechanism live once, in the detail file —
  scattering policy across sibling messages is context-poisoning.
- Canonical map + per-rule detail live in the `## Engineering Standards` index and
  `.agents/rules/<slug>.md`; canonical code lives in `.agents/snippets/`.

## Engineering Standards

> **Retrieval-led, not training-led.** Prefer this repo's standards over training-default
> patterns. Before writing code that touches a rule below, open its `@.agents/rules/<slug>.md`
> detail file and follow the house pattern. Full convention + SSOT:
> `docs/engineering-standards.md`.

This is the **dogfooded subset** for this repo — a pure-Python Cyclopts CLI (no API, DB, or web
layer). `[ast-grep]`/`[prek]` rules are enforced by `prek`; `[judgment]` rules are agent/reviewer
judgment; `[snippet]` ships canonical code under `.agents/snippets/` (in target repos).

### Standards

- **CES-79 · no raw dicts at boundaries** `[ast-grep]` — return/annotate a `@dataclass` or
  `BaseModel`, never a raw `dict`. → `@.agents/rules/no-dict.md`
- **CES-71 · keep files small** `[prek]` — `file-size-guard` warns at 400 lines, errors at 700.
  → `@.agents/rules/file-size-guard.md`
- **CES-107 · track the skills manifest** `[script]` — commit `skills-lock.json`; `.agents/skills/`
  is derived and gitignored; install restores from it. → `@.agents/rules/skills-manifest.md`
- **CES-45 · use the house get_logger** `[ast-grep]` — no direct `logging.getLogger`. →
  `@.agents/rules/log-get-logger.md`
- **CES-46 · libraries log, they don't print** `[ast-grep]` — no `print()` in library code;
  CLI/`__main__` exempt. → `@.agents/rules/log-no-print.md`
- **CES-74 · the house logger** `[snippet]` — structlog `core/logger.py` drop-in. →
  `@.agents/rules/core-logger.md`
- **CES-67 · typed, declarative CLIs** `[ast-grep]` — Typer/Cyclopts + Rich, not
  `argparse`/`click`/`sys.argv`. This repo uses Cyclopts. → `@.agents/rules/cli-typed-framework.md`
- **CES-63 · no catch-all modules** `[prek]` — no `utils.py`/`helpers.py`/`misc.py` (outside
  `tests/`). → `@.agents/rules/no-utils.md`
- **CES-32 · keep non-code out of the package** `[prek]` — no notebooks/`resources/`/`reports/`/`data/`
  inside the `scaffolding` package. → `@.agents/rules/repo-shape.md`
- **CES-75 · Conventional Commits** `[prek]` — commit subjects + PR titles follow
  `type(scope): description` (enforced here via the commit-msg hook + CI). →
  `@.agents/rules/agents-conventional-commits.md`
- **CES-77 · version pin** `[judgment]` — `requires-python` stays a deliberate local choice;
  house default is 3.14. → see `pyproject.toml`.
- **CES-5 · layered import direction** `[judgment]` — imports flow one way through the layers
  (`cli → engine → components/plan → templates_registry`), never upward. →
  `@.agents/rules/import-linter.md`
- **CES-16 · architectural vocabulary** `[judgment]` — name units with house terms, not ad-hoc
  synonyms. → `@.agents/rules/arch-vocabulary.md`
- **CES-8 · separate orchestration from logic** `[judgment]` — keep control flow thin; push logic
  and I/O into named units. → `@.agents/rules/spaghetti-mixed-orchestration.md`
- **CES-30 · respect the local repo** `[judgment]` — existing deliberate choices win over house
  defaults (the engine is clean-adds-only for this reason). →
  `@.agents/rules/general-respect-local-repo.md`
- **CES-58 · one modern lint stack** `[judgment]` — ruff + pyrefly + ast-grep via prek; no
  black/isort/flake8/pylint. → `@.agents/rules/py-legacy-lint-stack.md`
- **CES-64 · test against in-memory adapters** `[judgment]` — fakes over mocks. →
  `@.agents/rules/test-in-memory-adapters.md`
- **CES-65 · test through the interface** `[judgment]` — assert behaviour via the public seam
  (the `plan()`/`build_plan()` API), not internals. → `@.agents/rules/test-through-interface.md`
- **CES-66 · coverage gaps are a signal** `[judgment]` — an untested branch is a missing test or
  dead code, not a number to game. → `@.agents/rules/test-coverage-gap.md`
- **CES-91 · no AI co-authorship in commits** `[prek]` `[ci]` — no `Co-authored-by:`/`Generated
  with`/etc AI-attribution trailers; commit-msg hook + `commit-policy.yml` CI. →
  `@.agents/rules/no-ai-coauthorship.md`
- **CES-109 · dependency hygiene** `[prek]` — declared deps must match the import graph
  (`deptry`); `review`/`scaffolding/templates`/`skills` excluded (separate distribution / bundled
  payload). → `@.agents/rules/dep-hygiene-deptry.md`
- **CES-110 · cognitive complexity** `[prek]` — cognitive-complexity ceiling of 15 per function
  (`complexipy`), distinct from ruff's cyclomatic `C90`. →
  `@.agents/rules/cognitive-complexity-complexipy.md`
- **CES-111 · randomize test order** `[dependency]` — `pytest-randomly` runs every suite in a
  random, reproducibly-seeded order. → `@.agents/rules/test-order-randomization-pytest-randomly.md`
- **CES-113 · dependency review on PRs** `[ci]` — `dependency-review-action`, summary-only, not a
  merge gate. → `@.agents/rules/dependency-review-action.md`
- **CES-118 · no copy-paste duplication** `[prek]` — `jscpd`; `guide.md` exempted (deliberately
  mirrors `README.md`). → `@.agents/rules/code-duplication-jscpd.md`
- **CES-119 · dependency vulnerability scanning** `[ci]` — `osv-scanner` on `uv.lock`; replaces
  the old `pip-audit.yml`. → `@.agents/rules/osv-scanner-replace-pip-audit.md`

### Excluded here (don't apply to a pure-Python CLI)

- **CES-4 · api-schemas-extra-forbid** — no API request/response schemas in this repo.
- **CES-18 · arch-database-package** — no relational persistence layer.
- **CES-76 · settings-module** — no `BaseSettings` config surface (CLI reads flags via Cyclopts).
- **CES-17 · api-boundary-layout** — no inbound HTTP/`api` package.
- **CES-114 · hadolint-dockerfile-lint** — no root `Dockerfile` / `docker` CI component in this
  repo.

## Domain docs

Single-context — `CONTEXT.md` + `docs/adr/` at the repo root (created lazily).
See `docs/agents/domain.md`.

## Repo Workspace Defaults

- Use `.scratch/` for temporary plans, prd issue drafts, and disposable notes.
- Use `.tmp/` for generated local artifacts that should not be committed.
- Use `.worktrees/` for local Git worktrees when needed.
- Use `.journals/` for local/private session journals when using the `journalist` skill.
- Preserve unrelated user changes and avoid destructive Git commands unless explicitly requested.
