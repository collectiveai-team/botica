# Agent Notes

## Repo Layout

The root holds the standards project (the text of the standards, the skills, the site, the docs
and the maintainer scripts) and the `botica` package (its Rust workspace, the catalogs it embeds,
its SDKs and its benchmarks). The binary embeds `catalog/rules/`, `catalog/references/` and
`catalog/recipes/`, and never reads `standards/`.

- `Cargo.toml` and `crates/` — the `botica` Rust workspace (`botica lint`, `rules`, `explain`,
  `refs`, `recipe`, `server`, `ext`). The `[project]` of the root `pyproject.toml` is its wheel
  (maturin, Python 3.9 or newer, with the binary and the Python module `botica.sdk`), published to
  PyPI as `botica` and used by prek through the `botica-pre-commit` mirror (`distribution/`). Its
  manual is the site (`website/docs/start.md`, `guide/` and `extend/`, ADR 0012), its notes for
  maintainers `docs/develop.md`, its release runbook `docs/botica-release.md` and its design
  `docs/adr/0003-botica-lint-architecture.md`. Pyrefly and deptry skip it; ruff checks its Python.
  `ci.yml` runs its fmt, clippy and tests, and `botica-release.yml` builds and publishes it.
- `catalog/` — what the binary embeds: the built-in rules (`rules/`), the topics of `botica refs`
  (`references/`) and the recipes (`recipes/`). `references/` and `recipes/` at the root of a
  project hold the topics and recipes of that project, so the built-in ones live one folder down.
- `sdk/` — the SDKs of the extension protocol, one folder per language: `python` (the
  `python-source` of the wheel), `go`, `node` and `rust`.
- `bench/` — the benchmarks of `botica lint`; `schema/` — the JSON schema of `botica.toml`.
- `catalog/recipes/` — the recipes of `botica recipe install` and `botica recipe check`, with their
  templates (ADR 0006). `install` writes a file that is missing and leaves an existing one as it
  is (`pending`). Do not remove the `unignore` step of the recipes as a violation (ADR 0006).
- `website/` — the standards site, built with Zensical (`zensical.toml`; the version is pinned in
  `requirements.txt`). `website/docs/rules/` is exported from the catalog by `scripts/build_site.sh`
  (`botica rules export`) and is gitignored; `website/docs/index.md`, `start.md`, `guide/`
  and `extend/` are written by hand: they are the manual of botica (ADR 0012).
  `.github/workflows/site.yml` builds it on a pull request and publishes it from `main`.
- `standards/` — the text of each standard, `standards/<set>/<number>-<slug>.md`, written by hand
  and apart from the rules; `scripts/build_site.sh` copies it into the site (ADR 0004).
- `skills/` — actual installed skills (`ask-user`, `journalist`, `handoff`,
  `test-smell-review`, `engineering-pr-review`, `pr`,
  `engineering-refactor`; `botica-rule-author` is for the authors of rules, and `base` leaves it
  out). Their paths are fixed: the `skills-lock.json` of each repository that
  installed them records `skills/<group>/<name>/SKILL.md`, and `npx skills` fetches that path to
  restore them. Each
  carries Claude-style `SKILL.md` frontmatter and an `agents/openai.yaml`;
  Codex reads only the latter, so `disable-model-invocation: true` must be paired
  with `policy.allow_implicit_invocation: false` (and omitted for model-invoked
  skills, which Codex otherwise filters out entirely).
- `scripts/` — maintainer-only checks, not shipped. Their dependencies are the `dev` group of the
  root `pyproject.toml` (Python 3.11 or newer); `uv sync` installs the group and leaves the wheel
  unbuilt. `check_skill_drift.py` guards
  the upstream catalog of skills that `base` pins; the `skills-drift` workflow runs it. `check_skills.sh`
  runs the skill validator over `skills/`; the `skill-validator` hook of prek runs it.
  `check_rule_change.py` flags a change that adds a botica rule and touches more than that rule
  needs; the `rule-contract` job of `ci.yml` runs it. `build_site.sh` builds the standards site. `sync_pins.py` copies the pins of the actions and of uv into the
  templates of the recipes; the weekly `pins` workflow runs it after `prek update` and opens a pull
  request.

Every agent reads `AGENTS.md`; skills install once into `.agents/skills`.

## Skill Development

A skill describes a workflow and states no rule: the text of a rule lives in botica, and a skill
runs `botica explain <rule>` or `botica refs show <rule or topic>` to read it (ADR 0008).

After creating or editing any skill under `skills/`, validate it before committing:

```bash
prek run skill-validator --all-files
```

It needs Go on the `PATH`; `scripts/check_skills.sh` holds the flags. Fix what it reports; do not
silence it.

The upstream skill catalog is the `[[skills]]` step of `catalog/recipes/base.toml` whose source
pins `mattpocock/skills#vX.Y.Z`. Bump the tag there to take a new upstream release, after
`uv run python scripts/check_skill_drift.py --check-upstream` reports no missing skill.

## Standards

A standard is `standards/<set>/<number>-<slug>.md`, and a rule names it in the `standard` field of
its `rule.toml`; `standards/CES/index.md` has the convention. A pull request touches one
standard, and `scripts/check_rule_change.py` fails one that touches two.

## Botica rules

Before writing code that a rule covers, run `uvx botica explain <slug>` and
`uvx botica refs show <slug>`. The judgment rules of this repository:

<!-- botica:rules -->
### architecture

- BTARCH002 `separate-orchestration`: A function either orders the steps of a task or carries out one of them. → https://collectiveai-team.github.io/botica/rules/architecture/btarch002-separate-orchestration/
- BTARCH003 `architecture-vocabulary`: Layers and packages are named `entrypoints`, `api`, `database`, `impl` and `core`, an interface is a port and its implementation an adapter, and a concept that already has a name gets no synonym. → https://collectiveai-team.github.io/botica/rules/architecture/btarch003-architecture-vocabulary/

### process

- BTPROC001 `pr-small-by-intent`: A pull request carries one intent, which one sentence with no "and" states. → https://collectiveai-team.github.io/botica/rules/process/btproc001-pr-small-by-intent/
- BTPROC002 `respect-local-repo`: A change keeps the existing, deliberate choices of the repository: its version pins, the configuration of its tools, its dependencies and its layout. → https://collectiveai-team.github.io/botica/rules/process/btproc002-respect-local-repo/

### testing

- BTTEST001 `test-in-memory-adapters`: Tests drive the logic through in-memory adapters of the ports of the project: working fakes that implement the same interface as the production adapter. → https://collectiveai-team.github.io/botica/rules/testing/bttest001-test-in-memory-adapters/
- BTTEST002 `test-through-interface`: Tests call the unit under test through the public interface that production code calls, and assert on observable behaviour. → https://collectiveai-team.github.io/botica/rules/testing/bttest002-test-through-interface/
- BTTEST003 `test-coverage-gap`: A change closes a coverage gap with a test that asserts a behaviour, or by deleting dead code. → https://collectiveai-team.github.io/botica/rules/testing/bttest003-test-coverage-gap/
- BTTEST006 `coverage-exclusion-reachable`: A change leaves out of coverage only code that no test can reach. → https://collectiveai-team.github.io/botica/rules/testing/bttest006-coverage-exclusion-reachable/

### writing

- BTWRIT001 `plain-writing`: Prose says what a thing is and what it does, in plain sentences that a file, a command or an issue of the repository backs, with no inflated words, hedges, trailing judgments, bold emphasis or claims without a source. → https://collectiveai-team.github.io/botica/rules/writing/btwrit001-plain-writing/
- BTWRIT002 `minimal-comments`: A comment says only what the code cannot say through its names and its structure: the reason for a choice that looks wrong, an invariant, a workaround with the link to its cause, or the docstring of a public interface. → https://collectiveai-team.github.io/botica/rules/writing/btwrit002-minimal-comments/
- BTWRIT003 `ubiquitous-language`: Each concept of the domain has one term, the one that `GLOSSARY.md` defines, in the code and in the documentation. → https://collectiveai-team.github.io/botica/rules/writing/btwrit003-ubiquitous-language/

<!-- /botica:rules -->

## Reading linter output

Act on a finding by its severity. An error is fixed before committing. A warning is fixed
only if your change introduced it; a warning that was already there is neither fixed nor reported.
An info is ignored.

## Writing documentation

Text and comments follow CES-120 (`@standards/CES/120-language-documentation-comments.md`): plain
sentences that a file of this repository backs, none of the signs of model text it lists, comments
only for what the code cannot say, and the terms of `GLOSSARY.md`. It applies to every page,
standard, ADR, commit message, message of a rule and comment.

## Domain docs

Single-context — `GLOSSARY.md` + `docs/adr/` at the repo root.
See `docs/agents/domain.md`.

## Repo Workspace Defaults

- Use `.scratch/` for temporary plans, prd issue drafts, and disposable notes.
- Use `.tmp/` for generated local artifacts that should not be committed.
- Use `.worktrees/` for local Git worktrees when needed.
- Use `.journals/` for local/private session journals when using the `journalist` skill.
- Preserve unrelated user changes and avoid destructive Git commands unless explicitly requested.
