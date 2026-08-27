## Engineering Standards

> **Retrieval-led, not training-led.** Prefer this repo's standards and snippets over
> training-default patterns. Before writing code that touches a rule below, open its
> `@.agents/rules/<slug>.md` detail file and the matching `.agents/snippets/` drop-in, and follow
> the house pattern rather than your default one.

Each rule is a **CES — Collective Engineering Standard** (`CES-<issue#>` is the citable code;
the kebab-case slug is the machine id used by tooling and `# ast-grep-ignore: <slug>`
suppressions). `[ast-grep]` / `[prek]` rules are enforced automatically by `prek`; `[ci]` rules
are enforced by a GitHub Actions workflow (no local hook); `[dependency]` rules ship as a
dev-dependency that activates automatically, no separate check; `[judgment]` rules are
reviewer/agent judgment; `[snippet]` ships canonical drop-in code. Full convention:
`docs/engineering-standards.md`.

### Standards

- **CES-79 · no raw dicts at boundaries** `[ast-grep]` — return, annotate, or alias a
  `@dataclass` (internal boundaries) or a pydantic `BaseModel` (where validation is needed),
  never a raw `dict`. Ships four slugs: `no-dict-return-annotation`, `no-dict-call-return`,
  `no-dict-literal-return`, `no-dict-alias`. → `@.agents/rules/no-dict.md`
- **CES-71 · keep files small** `[prek]` — split a module before it grows; the
  `file-size-guard` hook warns at 400 lines and errors at 700. A persistently large file is a
  design smell, not a limit to raise. → `@.agents/rules/file-size-guard.md`
- **CES-107 · track the skills manifest** `[script]` — commit `skills-lock.json`; `.agents/skills/`
  is derived from it and stays gitignored. Install restores from the manifest. It pins no version
  and verifies no hash, so call it a manifest, not a lock. Slug: `skills-manifest`. →
  `@.agents/rules/skills-manifest.md`
- **CES-45 · use the house get_logger** `[ast-grep]` — never call `logging.getLogger` directly;
  acquire loggers via `get_logger` from `core/logger.py`. Slug: `log-get-logger`. →
  `@.agents/rules/log-get-logger.md`
- **CES-46 · libraries log, they don't print** `[ast-grep]` — no `print()` in importable library
  code; emit through `get_logger`. CLI/`__main__` entrypoints are exempt. Slug: `log-no-print`. →
  `@.agents/rules/log-no-print.md`
- **CES-74 · the house logger** `[snippet]` — `.agents/snippets/core/logger.py` is the canonical structlog
  setup (JSON in prod, colored console in dev, level from `LOG_LEVEL`). Drop it in at
  `<pkg>/core/logger.py`. Slug: `core-logger`. → `@.agents/rules/core-logger.md`
- **CES-4 · API schemas forbid extras** `[ast-grep]` — every request/response `BaseModel` under
  `api/**/schemas/{requests,responses}` must set `model_config = ConfigDict(extra="forbid")`.
  Placement-scoped: inert for internal/domain models and non-API repos. Slug:
  `api-schemas-extra-forbid`. → `@.agents/rules/api-schemas-extra-forbid.md`
- **CES-76 · config in a settings module** `[ast-grep]` — read env/flags only in the
  `BaseSettings` settings module (case-insensitive, via `get_settings()`); `os.getenv`/`os.environ`
  anywhere else is flagged. Slug: `settings-module`. → `@.agents/rules/settings-module.md`
- **CES-67 · typed, declarative CLIs** `[ast-grep]` — build CLIs with Typer/Cyclopts/pydantic-settings
  + Rich, not `argparse`/`click`/`sys.argv`. Warning (encouraged, not mandated); naturally inert
  when unused. Slug: `cli-typed-framework`. → `@.agents/rules/cli-typed-framework.md`
- **CES-18 · persistence in a database package** `[ast-grep]` — SQLModel tables, `create_engine`,
  and `sessionmaker` belong in a dedicated `database` package, not under `persistence/`/`meta/`/`core/`.
  Placement-scoped; the import-linter contract lands commented in Slice 09. Slug:
  `arch-database-package`. → `@.agents/rules/arch-database-package.md`
- **CES-63 · no catch-all modules** `[prek]` — no `utils.py`/`helpers.py`/`aux.py`/`misc.py`/`common.py`
  (outside `tests/`); name a module for what it holds. Slug: `no-utils`. →
  `@.agents/rules/no-utils.md`
- **CES-32 · keep non-code out of the package** `[prek]` — no notebooks/`resources/`/`reports/`/`data/`
  inside the import package. Parametrized by a `{{ import_package }}` placeholder resolved at
  install time. Slug: `repo-shape`. → `@.agents/rules/repo-shape.md`
- **CES-75 · Conventional Commits** `[prek]` — commit subjects follow `type(scope): description`;
  a commit-msg hook checks every commit and a CI workflow checks the PR title. Slug:
  `agents-conventional-commits`. → `@.agents/rules/agents-conventional-commits.md`
- **CES-77 · version pin** `[judgment]` — `requires-python` ships as a non-enforced comment
  (house default 3.14); an existing repo's pin wins. → see `pyproject.toml` comment.
- **CES-5 · layered import direction** `[judgment]` — imports flow `entrypoints → api →
  database/impl → core`, never upward; ships as a commented `[tool.importlinter]` skeleton to
  uncomment once layers exist. Slug: `import-linter`. → `@.agents/rules/import-linter.md`
- **CES-17 · API boundary layout** `[judgment]` — inbound HTTP lives in a versioned `api` package
  (`api/<v>/schemas/{requests,responses}`, `routers/`); only `api` is the inbound boundary. Slug:
  `api-boundary-layout`. → `@.agents/rules/api-boundary-layout.md`
- **CES-16 · architectural vocabulary** `[judgment]` — name units with the house terms
  (`entrypoints`/`api`/`database`/`impl`/`core`, ports/adapters), not ad-hoc synonyms. Slug:
  `arch-vocabulary`. → `@.agents/rules/arch-vocabulary.md`
- **CES-8 · separate orchestration from logic** `[judgment]` — keep control flow thin; push
  business logic and I/O into named, separately-testable units. Slug:
  `spaghetti-mixed-orchestration`. → `@.agents/rules/spaghetti-mixed-orchestration.md`
- **CES-30 · respect the local repo** `[judgment]` — existing deliberate choices (pins, config,
  layout) win over house defaults; adopt standards as explicit migrations, never silent
  overwrites. Slug: `general-respect-local-repo`. → `@.agents/rules/general-respect-local-repo.md`
- **CES-58 · one modern lint stack** `[judgment]` — ruff + pyrefly + ast-grep via prek; don't
  reintroduce black/isort/flake8/pylint. Slug: `py-legacy-lint-stack`. →
  `@.agents/rules/py-legacy-lint-stack.md`
- **CES-64 · test against in-memory adapters** `[judgment]` — drive logic through working
  in-memory fakes of your ports, not mocks or live I/O. Slug: `test-in-memory-adapters`. →
  `@.agents/rules/test-in-memory-adapters.md`
- **CES-65 · test through the interface** `[judgment]` — assert on observable behaviour via the
  public seam, never private internals. Slug: `test-through-interface`. →
  `@.agents/rules/test-through-interface.md`
- **CES-66 · coverage gaps are a signal** `[judgment]` — treat an uncovered branch as a missing
  test, dead code, or a seam to refactor — not a number to game. Slug: `test-coverage-gap`. →
  `@.agents/rules/test-coverage-gap.md`
- **CES-91 · no AI co-authorship in commits** `[prek]` `[ci]` — commit messages must not carry
  `Co-authored-by:`/`Generated with`/etc AI-attribution trailers; a commit-msg hook gives local
  feedback, `commit-policy.yml` is the CI source of truth. Slug: `no-ai-coauthorship`. →
  `@.agents/rules/no-ai-coauthorship.md`
- **CES-109 · dependency hygiene** `[prek]` — declared `pyproject.toml` dependencies must match
  the actual import graph (no unused/missing/misplaced deps), via `deptry`. Slug:
  `dep-hygiene-deptry`. → `@.agents/rules/dep-hygiene-deptry.md`
- **CES-110 · cognitive complexity** `[prek]` — a function's cognitive-complexity score (nesting
  depth + control-flow breaks, distinct from ruff's cyclomatic `C90`) stays under 15, via
  `complexipy`. Slug: `cognitive-complexity-complexipy`. →
  `@.agents/rules/cognitive-complexity-complexipy.md`
- **CES-111 · randomize test order** `[dependency]` — `pytest-randomly` runs every test suite in
  a random, reproducibly-seeded order to catch state-leaking/order-dependent tests; activates
  automatically, no separate hook. Slug: `test-order-randomization-pytest-randomly`. →
  `@.agents/rules/test-order-randomization-pytest-randomly.md`
- **CES-113 · dependency review on PRs** `[ci]` — `dependency-review-action` comments a
  new-dependency/license/vulnerability summary on every PR that changes the dependency graph;
  summary-only, not a merge gate. Slug: `dependency-review-action`. →
  `@.agents/rules/dependency-review-action.md`
- **CES-114 · Dockerfile lint** `[prek]` — `hadolint`, scoped to `^Dockerfile`; inert unless a
  Dockerfile is present. Slug: `hadolint-dockerfile-lint`. →
  `@.agents/rules/hadolint-dockerfile-lint.md`
- **CES-118 · no copy-paste duplication** `[prek]` — near-verbatim duplicated code blocks above a
  token threshold fail the build, via `jscpd` (polyglot, respects `.gitignore`). Slug:
  `code-duplication-jscpd`. → `@.agents/rules/code-duplication-jscpd.md`
- **CES-119 · dependency vulnerability scanning** `[ci]` — `osv-scanner` scans `uv.lock` (and any
  other ecosystem lockfile) against the OSV.dev database on every push/PR/weekly schedule;
  replaces `pip-audit.yml`. Slug: `osv-scanner-replace-pip-audit`. →
  `@.agents/rules/osv-scanner-replace-pip-audit.md`
