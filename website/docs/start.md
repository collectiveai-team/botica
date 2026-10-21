---
title: Start
---

# Start

Botica is a linter and a set of recipes for the repositories of a team. `botica lint` runs the
rules of its catalog over the files of a project, and the catalogs that the project adds. A rule
checks one [standard](standards/index.md). `botica recipe install` adds what a kind of repository
needs to a repository: the selection of rules, the hooks of prek, the workflows of CI, the agent
skills and the lines of `.gitignore`.

`botica lint` never calls a model or the network. It reads each file once and runs its rules in
parallel. A rule that no deterministic check can express is a judgment rule: botica exports it
for a judge, [abide](extend/rules.md#judgment-rules), and filters what the judge reports.

## Install

Botica is one binary. PyPI publishes it as the wheel `botica`, which also holds the module
`botica.sdk`. The binary runs without Python.

=== "Run once"

    ```bash
    uvx botica lint
    ```

=== "Install"

    ```bash
    uv tool install botica
    botica lint
    ```

=== "From source"

    ```bash
    cargo build --release -p botica-cli   # at the root of a checkout
    target/release/botica lint
    ```

## A first run

```bash
botica lint                    # the active rules over the current directory
botica rules --all             # every rule, whether it is active, and why
botica explain no-utils        # one rule: what it asks, how it is built, how it is configured here
```

`botica lint` exits with 0 when no finding is an error, with 1 when one is, and with 2 when the run
itself fails. [Lint](guide/lint.md) lists the options.

## Set up a repository

From the root of the repository, with [`uv`](https://docs.astral.sh/uv/) on the `PATH`, and
`npx` for the recipes that install skills:

```bash
botica recipe list                                     # the recipes and their steps
botica recipe install base base-python agent-claude-code ci-python repo-public
botica recipe check base base-python agent-claude-code ci-python repo-public
```

`install` writes what is missing and leaves an existing file as it is: it reports the file as
`pending` and exits with 1. It writes the rules of the recipes to the `select` of a new
`botica.toml`. Commit `skills-lock.json` afterwards: `.agents/skills/` is gitignored, and
`npx skills experimental_install` restores it from the manifest. [Recipes](guide/recipes.md)
describes each step.

## Next

- [Configuration](guide/configuration.md): `botica.toml`, the selection and the settings of a rule.
- [Findings](guide/findings.md): silence a finding, and keep the old ones in a baseline.
- [prek and CI](guide/prek-and-ci.md): run botica on each commit and each pull request.
- [Editor](guide/editor.md): the findings in the editor, through `botica server`.
- [Catalogs of a project](extend/catalogs.md): the rules of a project, in `.botica/rules/`.
