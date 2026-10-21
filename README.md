<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/collectiveai-team/botica/main/assets/botica-dark.png" />
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/collectiveai-team/botica/main/assets/botica-light.png" />
    <img src="https://raw.githubusercontent.com/collectiveai-team/botica/main/assets/botica-light.png" alt="Botica: recetas, alquimia y artilugios" width="600" style="max-width:100%; height:auto;" />
  </picture>
</p>

<p align="center">
  <strong><em>Botica: recetas, alquimia y artilugios</em></strong><br />
  <sub>Recipes and standards for projects: deterministic rules, templates, and design decisions.</sub>
</p>

<p align="center">
  <a href="https://github.com/collectiveai-team/botica/releases"><img alt="Release" src="https://img.shields.io/github/v/release/collectiveai-team/botica?logo=github" /></a>
  <a href="https://pypi.org/project/botica/"><img alt="PyPI" src="https://img.shields.io/pypi/v/botica?logo=pypi&logoColor=white" /></a>
  <a href="https://opencode.ai/"><img src="https://img.shields.io/badge/Agents-opencode%20%C2%B7%20claude--code%20%C2%B7%20codex-1d1d1d?logo=anthropic&logoColor=white" alt="opencode · claude-code · codex"></a>
  <a href="https://github.com/collectiveai-team/botica/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT"></a>
</p>

---

**Documentation**: [https://collectiveai-team.github.io/botica/](https://collectiveai-team.github.io/botica/)

**Source Code**: [https://github.com/collectiveai-team/botica](https://github.com/collectiveai-team/botica)

---

Botica is a linter and a set of recipes for the repositories of a team.

- **`botica lint`** runs rules over the files of a project: the built-in catalog, and the rules
  that the project writes in `.botica/rules/`. Each rule checks a written
  [standard](https://collectiveai-team.github.io/botica/standards/). It never calls a model or the
  network, and it runs as a command, a hook of prek, a job of CI and a language server.
- **`botica recipe install`** sets up a repository of some kind: the selection of rules in
  `botica.toml`, the hooks of prek, the workflows of GitHub Actions, the agent skills and the
  lines of `.gitignore`. It adds what is missing, and a file that exists is not replaced.
- **Judgment rules** cover what no deterministic check can express. Botica exports them to
  `AGENTS.md` and to the rubric of [abide](https://www.npmjs.com/package/@coldtea/abide), the judge,
  and filters what the judge reports with the same configuration and comments.

Most rules read Python, TypeScript and JavaScript, a few read Go and Rust, and the rest read the
files of any project: their names, their size, the layout of the folders, the commit messages.

## Install

Botica is one binary, published on PyPI as the wheel `botica`. It runs without Python.

```bash
uvx botica lint            # run it once, without installing
uv tool install botica     # or keep it installed
```

From a checkout: `cargo build --release -p botica-cli`, and the binary is `target/release/botica`.

## Use

```bash
botica lint                    # the active rules over the current directory
botica lint --diff origin/main # only the findings on the lines that a branch adds
botica rules --all             # every rule, whether it is active, and why
botica explain no-utils        # what a rule asks, and how it is configured here
```

`botica lint` exits with 1 when a finding is an error and with 2 when the run fails. A
configuration is `botica.toml`, or `[tool.botica]` in `pyproject.toml`:

```toml
select = ["file-size-guard", "no-utils", "log-no-print"]

[rules.file-size-guard]
warn = 500
max = 900
```

A comment silences one finding, with its reason:

```python
print(report)  # noqa: BTLOG002 -- the output of the command
```

The [guide](https://collectiveai-team.github.io/botica/guide/lint/) has every command and key,
the [baseline](https://collectiveai-team.github.io/botica/guide/findings/#known-findings) of known
findings, the [editor](https://collectiveai-team.github.io/botica/guide/editor/) (Neovim, Helix,
Emacs) and the [hooks and CI](https://collectiveai-team.github.io/botica/guide/prek-and-ci/).

## Set up a repository

From the root of the repository, with `uv` and `npx` on the `PATH`:

```bash
uvx botica recipe install base base-python agent-claude-code ci-python repo-public
```

| Recipe | What it adds |
|---|---|
| `base` | any repository: the rules about any file, the skills, `.gitignore`, the hooks of prek, the workflow of abide, the sections of `AGENTS.md` |
| `base-python` | a Python project: the rules about Python code and `pyproject.toml`, the Python hooks and the dev tools |
| `agent-claude-code`, `agent-codex`, `agent-opencode` | the configuration and the skills of an agent |
| `ci`, `ci-python`, `ci-node`, `ci-go` | the workflows of GitHub Actions for commits and for an ecosystem |
| `repo-public`, `repo-private` | the workflows that fit the visibility of the repository |
| `ci-publish`, `ci-docker`, `ci-dependency-review`, `ci-automerge-public`, `ci-automerge-private`, `secrets-varlock` | one workflow or tool each |

`botica recipe list` prints them with their steps, and `botica recipe check` says how the
repository differs from them. A file that exists is left as it is and reported `pending`. Commit
`skills-lock.json` afterwards: `npx skills experimental_install` restores `.agents/skills/` from
it. [Recipes](https://collectiveai-team.github.io/botica/guide/recipes/) describes each step, and
a project adds its own recipes in `recipes/`.

## Extend

A project writes its rules in `.botica/rules/<catalog>/`, and `botica rules new` writes the folder
of one:

```bash
botica rules new structure/no-eval --kind ast-grep --language python --catalog .botica/rules/acme
botica rules conform .botica/rules/acme   # the rule over its fixtures
```

- A **pattern rule** is a YAML pattern over the syntax tree of a file.
- An **extension** is a program that answers the protocol of botica over stdin and stdout. The
  SDKs are [Python](https://github.com/collectiveai-team/botica/blob/main/sdk/python/README.md) (`botica.sdk`, in the wheel), [Node](https://github.com/collectiveai-team/botica/blob/main/sdk/node/README.md),
  [Go](https://github.com/collectiveai-team/botica/blob/main/sdk/go/README.md) and [Rust](https://github.com/collectiveai-team/botica/blob/main/sdk/rust/README.md).
- An **external rule** is enforced by another tool, and botica checks that its configuration is
  there.
- A **judgment rule** is a question for abide, calibrated with `botica rules test --judge abide`.

[Extend](https://collectiveai-team.github.io/botica/extend/catalogs/) has a first rule, its
fixtures, the protocol and the SDKs.

## This repository

| Folder | What it holds |
|---|---|
| `crates/` | the Rust workspace of `botica` |
| `catalog/` | what the binary embeds: the built-in rules, the topics of `botica refs`, the recipes |
| `standards/` | the text of each standard, `standards/<set>/<number>-<slug>.md`; the CES are the standards of Collective |
| `sdk/` | the SDKs of the extension protocol |
| `skills/` | the agent skills that the recipe `base` installs |
| `website/` | the site: the manual, the standards, the rules and the references |
| `distribution/` | the mirror `botica-pre-commit` for prek |
| `docs/adr/` | the design decisions |

A change to a standard is a pull request with the file of the standard and the folders of its
rules; the steps are in
[Propose or change a standard](https://collectiveai-team.github.io/botica/standards/#propose-or-change-a-standard).
[`docs/develop.md`](https://github.com/collectiveai-team/botica/blob/main/docs/develop.md) has the commands to build and test botica, and
[`docs/botica-release.md`](https://github.com/collectiveai-team/botica/blob/main/docs/botica-release.md) the release.

## The skills of this repository

The recipe `base` installs them with `npx skills add collectiveai-team/botica`, beside the
skills of Matt Pocock that its `[[skills]]` step pins to a tag. The skills are under `skills/`, and
the `[[skills]]` steps of `catalog/recipes/base.toml` list the ones it installs.

`npx skills add` and `skills update` overwrite the installed copies in place, so local edits go
in this repository's `skills/`, not in `.agents/skills`.

## License

MIT, in [`LICENSE`](https://github.com/collectiveai-team/botica/blob/main/LICENSE).
