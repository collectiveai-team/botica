---
title: Findings
---

# Findings

## Silence a finding

```python
x = compute()  # botica: ignore[some-rule]
# botica: ignore-file[file-size-guard]    <- anywhere in the file
print(x)  # noqa: BTLOG002 -- reason
```

A finding about a whole file (size, name, layout) has no line: use `ignore-file`, the rule's
`ignores`/`extend-ignores`, or `extend-exclude`. With ruff, put `lint.external = ["BT"]` in its
configuration so it does not report `RUF102` for `BT` codes (the section `# MARK: Ruff` of
`base-python` has it).

A rule of project scope (`external-config`, `import-layers`, `recipe-conflict`,
`settings-env-schema`) is silenced in
the configuration, with the path in `extend-ignores` of `[rules.<slug>]` or the rule in `ignore`; a
comment that names it is `invalid-suppression`. `external-config-weakened` is the exception: its
finding sits on a line of a file of settings (TOML, YAML, `.npmrc`), and a comment with a reason on
that line silences it, `# botica: ignore[external-config-weakened] -- <reason>`. In the INI files of
pytest the comment goes on a line of its own, as `ignore-file`. A judgment rule is silenced for a
whole file, with a reason: `# botica: ignore-file[<slug>] -- <reason>`.

The README of each rule, on its page under [Rules](../rules/index.md), shows the comment of each
language it reads.

## Known findings

A rule that measures a function, such as `cognitive-complexity`, can keep its old findings in a
baseline while the new ones fail
([ADR 0011](https://github.com/collectiveai-team/botica/blob/main/docs/adr/0011-native-cognitive-complexity.md)).
`baseline` in the configuration names the file, from the root of the project, and
`botica lint --write-baseline`, run at the root with no paths, writes it with the findings of the
run and exits with 0:

```toml
baseline = "botica-baseline.toml"
```

```toml
[[finding]]
rule = "cognitive-complexity"
path = "src/orders.py"
name = "Order::total"
value = 23
```

A finding whose rule, path and function match an entry is left out while its value is at most the
one of the entry; above it, it is reported with the value of the entry. A function that the file
does not list is reported. The baseline applies after the comments and the configuration, and
before the filter of `--diff`. A run fails when the file it names does not exist. `--write-baseline`
writes the file again from the run, so an entry whose finding is gone leaves it. The editor shows
every finding.
