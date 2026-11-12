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
`rule-dirs`, `catalogs`, `extensions`, `references`, `recipes`, `baseline`, `history`, `review`. Any of them can be given
for one run with `--config 'key = value'`.

- `select`, `extend-select` and `ignore` name rules by slug, by code (`BTLOG002`) or by the code of
  a family (`BTLOG`). `botica recipe install` writes `select` from the rules of the recipes, and
  the hooks of prek and the editor read it.
- `[rules.<slug>]` holds the settings of one rule; `botica explain <slug>` lists them with their
  values here.
- `extend-exclude` and `extend-include` change [which files](lint.md#which-files) the walk reaches.
- `rule-dirs`, `catalogs` and `extensions` are in [Catalogs of a project](../extend/catalogs.md)
  and [Extensions](../extend/extensions.md); `references` in [References](refs.md); `recipes` in
  [Recipes](recipes.md); `baseline` in [Known findings](findings.md#known-findings); `history` in
  [History of findings](lint.md#history-of-findings); `review` is [below](#review).

`code-dirs` says where the code of the project lives (`python`, `js`, `rust`, `go`, `jvm`), for
the layout rules. Without it they are detected from `pyproject.toml`, `package.json`,
`Cargo.toml`, `go.mod` and the like; `botica explain no-notebooks` lists what was found.

## Review

The `review` table says who answers for each path and how much review a change needs. The design
is [ADR 0018](https://github.com/collectiveai-team/botica/blob/main/docs/adr/0018-risk-tiers-and-classifier.md)
for the areas and the classifier and
[ADR 0017](https://github.com/collectiveai-team/botica/blob/main/docs/adr/0017-review-roles.md)
for the roles. botica reads and checks the table; the commands that use it come in later changes.

```toml
# botica.toml; in pyproject.toml the tables are [tool.botica.review] and [[tool.botica.review.areas]]
[review]
generated = ["/uv.lock", "schema/botica.schema.json"]

[review.classifier]
required-checks = ["test", "lint"]

[[review.areas]]
name = "platform"
paths = ["ci/**", "Dockerfile"]
owners = ["@org/platform"]

[[review.areas]]
name = "critical"
paths = ["src/**/auth/**", "db/migrations/**"]
owners = ["@org/security"]
approvals = 2

[[review.areas]]
name = "periphery"
paths = ["docs/**"]
```

An entry of `[[review.areas]]` has these keys:

| Key | Value | Default |
|---|---|---|
| `name` | `critical`, `platform`, `core` or `periphery` | required |
| `paths` | globs, written as in a `.gitignore` | required |
| `owners` | `@user`, `@org/team`, `@org/group/team` or an email address | none; required for `critical` and `platform` |
| `tier` | `low`, `medium` or `high` | the floor of the name |
| `approvals` | the approvals that the paths ask for | 1; 0 for `periphery` |

The floor of a name is `high` for `critical`, `medium` for `platform` and `core`, and `low` for
`periphery`. A name may come in several entries, each with its own paths and owners. A file that no
entry matches belongs to `core`. A project with no entries has no change at the `low` tier.

The configuration fails to load when:

- a `tier` is under the floor of its name, `approvals` is 0 in any area but `periphery`, or an
  entry of `critical` or `platform` has no `owners`;
- an owner is not `@user`, `@org/team`, `@org/group/team` or an email address, or `paths` is empty;
- a name of `[review.classifier]` `required-checks` is blank, `dir` is not a plain directory
  without wildcards, or a key of the table is unknown;
- a glob is blank, starts with `#` or `!`, has spaces around it or is invalid, or the same glob is
  listed twice, in one entry or in two;
- the table has entries and none is `platform`;
- an entry of `periphery` can select a path that is always `platform`:
  `botica.toml`, `pyproject.toml` at the root, `.github/review/**`, `.gitlab/review/**` and the
  folder of `dir`, `skills-lock.json`, `.github/workflows/**`, `.github/actions/**`,
  `.gitlab-ci.yml`, `.github/CODEOWNERS`, `.gitlab/CODEOWNERS`, `.gitattributes`, `prek.toml`,
  `.abide/**`, and at any depth `AGENTS.md`, `CLAUDE.md`, `.claude/**`, `.mcp.json`,
  `.cursorrules`, `INVARIANTS.md`, `REVIEW.md` and `SKILL.md`. The check compares the globs with
  each other and reads no file of the checkout, so `.github/workflows/*.yml` and
  `docs/**/AGENTS.md` fail too. An entry that ends in `**`, as `docs/**`, is accepted: the list
  keeps a forced file inside it as `platform`.

A glob with no `/` is read at every depth, as in a `.gitignore`: `README.md` also selects
`.github/review/README.md`, which is `platform`. Write `/README.md` for the file at the root.

The check compares each glob as a pattern. When it cannot tell whether two globs select the same
path, it refuses the configuration. A `periphery` folder is written `docs/**`: `docs`, `docs/` and
`/docs` can select the `AGENTS.md` inside the folder and fail, and so does a wildcard that may be a
folder, such as `/*.github`. A glob with a character class (`[...]`), an escape (`\`), more than 4
groups of braces or more than 32 commas fails in `periphery` and in `generated`. A name with a dot,
such as `docs/v1.0`, is read as a file.

The other keys of `[review]`:

| Key | Meaning | Default |
|---|---|---|
| `dir` | the folder of the review roles, from the root, as a plain directory with no wildcard | `.github/review` or `.gitlab/review` |
| `roles` | the roles that CI expects: `spec`, `security`, `tests`, `architecture`, `contracts`, `performance`, `refute`; `[]` expects none | all seven |
| `generated` | globs of generated files, which the size of a change does not count; one that can select a path of `critical`, `platform` or the forced list fails to load, so write `/uv.lock` and not `uv.lock` | none |

`[review.classifier]` holds the settings that ADR 0018 gives the classifier, which a later change
adds. A value out of its bound fails to load:

| Key | Meaning | Default | Bound |
|---|---|---|---|
| `mode` | `shadow` decides and writes the report and never merges; `enforce` merges | `shadow` | |
| `required-checks` | names of the checks that must be present and green; an empty list sends every change to a person | none | |
| `question-max` | a question answers `true` with a probability under this | 0.1 | 0 to 0.3 |
| `impact-min` | the probability that a mistake shows first to the team or to whoever develops the repository, at least | 0.9 | 0.7 to 1 |
| `min-confidence` | a `choice` answer with less confidence goes to a person | 0.7 | 0.7 to 1 |
| `max-changed-lines` | a larger change goes to a person | 400 | up to 1000 |
