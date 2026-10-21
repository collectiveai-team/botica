<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/collectiveai-team/botica/main/assets/botica-dark.svg" />
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/collectiveai-team/botica/main/assets/botica-light.svg" />
    <img src="https://raw.githubusercontent.com/collectiveai-team/botica/main/assets/botica-light.svg" alt="Botica: recetas, alquimia y artilugios" width="600" style="max-width:100%; height:auto;" />
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
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT"></a>
</p>

---

**Source Code**: [https://github.com/collectiveai-team/botica](https://github.com/collectiveai-team/botica)

---

This repository has:

- `standards/` — the text of each house standard, `CES-<number>` (see
  [`standards/CES/index.md`](standards/CES/index.md)).
- `crates/` and `catalog/` — `botica`, published on PyPI: `botica lint` measures the rules of the catalog, and
  `botica recipe` installs the recipes. Its manual is the [site](https://collectiveai-team.github.io/botica/start/).
- `skills/` — the agent skills of the house.
- `website/` — the site of the standards and the rules.

## Set up a repository

Run from the root of the repository, with [`uv`](https://docs.astral.sh/uv/) on the `PATH` (and
`npx` for the recipes that install skills):

```bash
uvx botica recipe list                                     # the recipes and their steps
uvx botica recipe install base base-python agent-claude-code ci-python repo-public
uvx botica recipe check base base-python agent-claude-code ci-python repo-public
```

`install` writes what is missing and leaves an existing file as it is: it reports the file as
`pending` and exits with 1. Commit `skills-lock.json` afterwards (CES-107); `.agents/skills/` is
gitignored and `npx skills experimental_install` restores it from the manifest.

`botica recipe list` prints the built-in recipes with what each installs. A project adds its
own recipes in `recipes/` (ADR [0006](docs/adr/0006-external-rules-and-recipes.md)).

## The skills of this repository

The recipe `base` installs them with `npx skills add collectiveai-team/botica`, beside the
skills of Matt Pocock that its `[[skills]]` step pins to a tag. The skills are under `skills/`, and
the `[[skills]]` steps of `catalog/recipes/base.toml` list the ones it installs.

`npx skills add` and `skills update` overwrite the installed copies in place, so local edits go
in this repository's `skills/`, not in `.agents/skills`.
