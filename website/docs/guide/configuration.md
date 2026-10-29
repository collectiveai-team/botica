---
title: Configuration
---

# Configuration

`botica.toml`, or `[tool.botica]` in `pyproject.toml`, found by walking up from the working
directory to the root of the git repository; a configuration above that root belongs to another
project. Defining both is an error, and so is an unknown key or rule. The JSON Schema is
[`schema/botica.schema.json`](https://github.com/collectiveai-team/botica/blob/main/schema/botica.schema.json).

```toml
# botica.toml
extend-exclude = ["vendor/"]
ignore = ["no-tests-in-package"]

[rules.file-size-guard]
warn = 500
max = 900
```

Keys: `select`, `extend-select`, `ignore`, `preview`, `languages`, `extend-exclude`,
`extend-include`, `code-dirs`, `rules` (settings per rule), `overrides`, `external`, `replace`,
`rule-dirs`, `catalogs`, `extensions`, `references`, `recipes`, `baseline`. Any of them can be given
for one run with `--config 'key = value'`.

- `select`, `extend-select` and `ignore` name rules by slug, by code (`BTLOG002`) or by the code of
  a family (`BTLOG`). `botica recipe install` writes `select` from the rules of the recipes, and
  the hooks of prek and the editor read it.
- `[rules.<slug>]` holds the settings of one rule; `botica explain <slug>` lists them with their
  values here.
- A code or a slug that a rule had in an older release still names it, with a warning on stderr
  that gives its current code and slug: ``botica: warning: `BTARCH005` is the old code of
  `db-no-create-all` (BTDB002 since 0.13.0)`` (ADR 0016). The options of the command line,
  `botica explain` and `botica refs show` print the same warning. A suppression comment that names
  it silences the rule, with no warning.
- A removed rule in `select`, `extend-select`, `--select`, `--extend-select` or with
  `enabled = true` stops `botica lint` with exit code 2, and the message names the release that
  removed it and the rule that replaces it. Anywhere else, and in `botica rules`, `botica explain`
  and `botica recipe check`, it is a warning.
- `extend-exclude` and `extend-include` change [which files](lint.md#which-files) the walk reaches.
- `rule-dirs`, `catalogs` and `extensions` are in [Catalogs of a project](../extend/catalogs.md)
  and [Extensions](../extend/extensions.md); `references` in [References](refs.md); `recipes` in
  [Recipes](recipes.md); `baseline` in [Known findings](findings.md#known-findings).

`code-dirs` says where the code of the project lives (`python`, `js`, `rust`, `go`, `jvm`), for
the layout rules. Without it they are detected from `pyproject.toml`, `package.json`,
`Cargo.toml`, `go.mod` and the like; `botica explain no-notebooks` lists what was found.
