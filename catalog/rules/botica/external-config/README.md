## Directive

An external rule is enforced by another tool, and the rule names the fragment of configuration the
tool needs and the file of the project it goes into (`config = "config/prek.toml"` in its
`rule.toml` goes into `prek.toml`). For each active external rule whose tool the project uses,
`botica lint` reads the settings of the tool at the root of the project and reports a file, as a
whole, when no file of settings of the tool exists, when the file cannot be read in its format
(TOML, YAML or INI), or when the settings lack a key of the fragment. The check runs on every
`botica lint`, whatever files it is given.

A project uses the tool of an external implementation when one of its files matches a glob of the
`applies-when` of the implementation (`pnpm-lock.yaml`, `**/Dockerfile`), or, without
`applies-when`, when it has a file of one of its `languages`. `*`, or a language whose files botica
does not know, always counts. A project with no Go file is not asked for the configuration of a
linter of Go, and one without `pnpm-lock.yaml` is not asked for the settings of pnpm.

botica reads the settings of ruff, pyrefly, pytest, uv, pnpm, npm, Yarn, Cargo and Bun where the
tool reads them, in the order the tool prefers the files, and follows `extend` of ruff and
`extends` of pyrefly inside the project:

| Tool | Files, the first that counts wins |
|---|---|
| ruff | `.ruff.toml`, `ruff.toml`, `pyproject.toml` with `[tool.ruff]` |
| pyrefly | `pyrefly.toml`, `pyproject.toml` with `[tool.pyrefly]` |
| pytest | `pytest.toml`, `.pytest.toml`, `pytest.ini`, `.pytest.ini`, `pyproject.toml` with `[tool.pytest]` or `[tool.pytest.ini_options]`, `tox.ini` with `[pytest]`, `setup.cfg` with `[tool:pytest]` |
| uv | `uv.toml`, `pyproject.toml` with `[tool.uv]` |
| pnpm | `pnpm-workspace.yaml` |
| npm | `.npmrc` |
| Yarn | `.yarnrc.yml` |
| Cargo | `.cargo/config`, `.cargo/config.toml` |
| Bun | `bunfig.toml` |

Any other tool, or a fragment for a file that is not one of these, is read from the file the
fragment names.

The settings lack a fragment when they do not set one of its keys, or when a list of the settings
lacks an item of a list of the fragment. A key that the settings set to a value that holds less than
the fragment asks is the decision of the project, and this rule does not report it: a shorter
cooldown, `preset = "basic"` where the fragment asks `strict`, `convention = "numpy"` where it asks
`google`. Some values hold in more ways, as the tool reads them:

- A code of ruff under `select` or `extend-select` of the fragment holds when the settings select
  it, with `select` or `extend-select`. The settings may name its family (`A` takes in `A005` and
  not `ASYNC100`) or every code of the family, one by one. A code that `ignore` or `extend-ignore`
  switches off is the decision of the project. Without `select`, ruff applies its default rules,
  which change between versions, and botica does not count them: the family has to be written.
  With `explicit-preview-rules = true`, a rule in preview is on only when the settings name its
  code, as ruff does: `select = ["DOC"]` does not hold `DOC102`. botica knows the codes of ruff
  0.17.1.
- `preset = "all"` of pyrefly holds `preset = "strict"`, and `strict` holds `default`. A kind of
  error under `errors` holds when the project sets it at least as high (`true` is `error`, `false`
  is `ignore`), or when the preset is `all`.
- In an INI file and in `[tool.pytest.ini_options]` a value is text: `"true"` holds `true`, and
  `addopts = "-ra --strict-markers"` holds `["--strict-markers"]`.
- A time span of a package manager (`exclude-newer` of uv, `minimumReleaseAge` of pnpm,
  `min-release-age` of npm, `npmMinimalAgeGate` of Yarn, `global-min-publish-age` of Cargo) holds
  with a span at least as long, written in any form the tool reads: `"1 week"` holds `"7 days"`, and
  `"14 days"` holds it too.

`botica explain <rule>` shows the configuration of an external rule.

## Why

A standard enforced by a tool holds only while the tool is set up: a repository that drops the hook
of the tool from `prek.toml` stops running the check, and no check fails. A repository that writes
the same configuration in another file or with another key, or that sets a check of the tool
weaker on purpose, has made a choice that this rule does not report.

## Suppression

The finding is about a whole file of configuration. Switch the external rule off, or this one with
`ignore = ["external-config"]` in the configuration of botica.
