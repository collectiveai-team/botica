---
title: Use
---

# Use

## Install

Botica is a single binary, published on PyPI as a wheel that needs no Python at run time.

=== "Bash"

    ```bash
    uv tool install botica     # keep it installed
    uvx botica lint            # or run it once, without installing
    ```

## Run

=== "Bash"

    ```bash
    botica lint                         # the current directory
    botica lint src tests/a.py          # directories to walk, or files
    botica lint --select file-size-guard,no-utils
    botica lint --output-format json    # text (default), json, sarif or github
    botica rules                        # which rules are active, and why (--all adds the inactive ones)
    botica explain no-notebooks         # what a rule is and how it is configured here
    ```

The exit code is `0` when the run is clean (a warning does not fail it), `1` when a finding has
the severity error, and `2` when the run itself failed.

## Configure

Botica reads `botica.toml`, or `[tool.botica]` in `pyproject.toml`, found by walking up from the
working directory to the root of the git repository. Defining both is an error, and so is an unknown key or rule.

=== "TOML"

    ```toml
    # botica.toml
    extend-exclude = ["vendor/"]
    ignore = ["no-tests-in-package"]

    [rules.file-size-guard]
    warn = 500
    max = 900
    ```

`botica explain <rule>` shows how a rule is configured in the project where you run it.

## Silence a finding

=== "Python"

    ```python
    x = compute()  # botica: ignore[some-rule]
    # botica: ignore-file[file-size-guard]    <- anywhere in the file
    print(x)  # noqa: BTLOG002 -- reason
    ```

=== "TypeScript"

    ```ts
    const x = compute(); // botica: ignore[some-rule]
    ```

A finding about a whole file (its size, its name, its layout) has no line: use `ignore-file`, the
`ignores` of the rule in the configuration, or `extend-exclude`.

A rule of project scope (`external-config`, `import-layers`, `settings-env-schema`) is silenced in
the configuration, with the path in `extend-ignores` of `[rules.<slug>]` or the rule in `ignore`. A
judgment rule is silenced for a whole file, with a reason after `--`.

## Run it on every commit

=== "TOML"

    ```toml
    # prek.toml
    [[repos]]
    repo = "https://github.com/collectiveai-team/botica-pre-commit"
    rev = "v0.11.0"

    [[repos.hooks]]
    id = "botica"
    args = ["--select", "file-size-guard,no-utils,log-no-print"]
    ```

The mirror installs the wheel of the same version, so `rev` pins the binary.
