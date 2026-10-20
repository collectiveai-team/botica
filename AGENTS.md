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
  manual is `docs/botica.md`, its release runbook `docs/botica-release.md` and its design
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
  is (`pending`). The one step that deletes text is `unignore`, which removes a literal line of
  `UNIGNORE_WHITELIST` (`skills-lock.json`) from `.gitignore` and reports it as `edited`. Keep the
  whitelist literal and do not remove the step as a violation (ADR 0006).
- `website/` — the standards site, built with Zensical (`zensical.toml`; the version is pinned in
  `requirements.txt`). `website/docs/rules/` is exported from the catalog by `scripts/build_site.sh`
  (`botica rules export`) and is gitignored; only `website/docs/index.md` is written by hand.
  `.github/workflows/site.yml` builds it on a pull request and publishes it from `main`.
- `standards/` — the text of each standard, `standards/<set>/<number>-<slug>.md`, written by hand
  and apart from the rules; `scripts/build_site.sh` copies it into the site (ADR 0004).
- `skills/` — actual installed skills (`ask-user`, `journalist`, `handoff`,
  `test-smell-review`, `engineering-pr-review`, `pr`,
  `engineering-refactor`; `botica-rule-author` is for the authors of rules, and `base` leaves it
  out). Their paths are fixed: the `skills-lock.json` of each repository that
  installed them records `skills/<group>/<name>/SKILL.md`, and `npx skills` fetches that path to
  restore them. Each
  carries Claude-style `SKILL.md` frontmatter **and** an `agents/openai.yaml`;
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

An agent is a recipe chosen by name: `agent-opencode` writes `opencode.jsonc`,
`agent-claude-code` writes `.claude/settings.json` and links `CLAUDE.md`→`AGENTS.md` and
`.claude/skills`→`../.agents/skills`, and `agent-codex` has no file. `AGENTS.md` is the shared
brain; skills install once into `.agents/skills`.

## Skill Development

A skill describes a workflow and states no rule: the text of a rule lives in botica, and a skill
runs `botica explain <rule>` or `botica refs show <rule or topic>` to read it (ADR 0008).

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

`tessl review` is not used: it asks for an interactive login.

The upstream skill catalog is the `[[skills]]` step of `catalog/recipes/base.toml` whose source
pins `mattpocock/skills#vX.Y.Z`. Bump the tag there to take a new upstream release, after
`uv run python scripts/check_skill_drift.py --check-upstream` reports no missing skill.

## Standards

A standard is `standards/<set>/<number>-<slug>.md`, and a rule names it in the `standard` field of
its `rule.toml`; `docs/engineering-standards.md` has the convention. A pull request touches one
standard, and `scripts/check_rule_change.py` fails one that touches two.

## Botica rules

Before writing code that a rule covers, run `uvx botica explain <slug>` and
`uvx botica refs show <slug>`. The judgment rules of this repository:

<!-- botica:rules -->
<!-- /botica:rules -->

## Reading linter output

Act on a finding by its severity. An **error** is fixed before committing. A **warning** is fixed
only if your change introduced it; a warning that was already there is neither fixed nor reported.
An **info** is ignored.

## Writing documentation

Text and comments follow CES-120 (`@standards/CES/120-language-documentation-comments.md`): plain
sentences that a file of this repository backs, none of the signs of model text it lists, comments
only for what the code cannot say, and the terms of `GLOSSARY.md`. It applies to every page,
standard, ADR, commit message, message of a rule and comment.

## Domain docs

Single-context — `GLOSSARY.md` + `docs/adr/` at the repo root (created lazily).
See `docs/agents/domain.md`.

## Repo Workspace Defaults

- Use `.scratch/` for temporary plans, prd issue drafts, and disposable notes.
- Use `.tmp/` for generated local artifacts that should not be committed.
- Use `.worktrees/` for local Git worktrees when needed.
- Use `.journals/` for local/private session journals when using the `journalist` skill.
- Preserve unrelated user changes and avoid destructive Git commands unless explicitly requested.
