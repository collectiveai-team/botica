## Directive

A value that the settings of a tool set and that holds less than an active external rule asks is a
warning, on the line of the value, until a comment there gives the reason: `preset = "basic"` of
pyrefly where the rule asks `strict`, `exclude-newer = "3 days"` of uv where it asks `7 days`, a code
of ruff that `ignore` switches off where the rule selects it. `external-config` decides what holds
less, reading the settings where the tool reads them, and does not report it: the value is the
decision of the project. This rule asks for the reason of that decision on the line of the value.

```toml
# pyproject.toml, with a rule that asks for preset = "strict"
[tool.pyrefly]
preset = "basic"  # reported here
```

The finding sits on the key and its value in the file the tool reads, or on the code inside
`ignore` for ruff. When botica does not find that line, as for a value in a file that `extend`
names, the finding is about the whole file. A rule whose tool the project does not use asks for
nothing.

## Why

A project sets a weaker value for a reason: a mirror that lags behind the registry, a package that
moves to strict types later. Without the reason next to the value, the next reader cannot tell a
decision from a mistake, and nobody sees the gap between what the standard asks and what the project
runs.

## Suppression

A comment with a reason after `--`, on the line of the value:

```toml
[tool.pyrefly]
preset = "basic"  # botica: ignore[external-config-weakened] -- the legacy package moves to strict in Q1
```

```yaml
# pnpm-workspace.yaml
minimumReleaseAge: 60  # botica: ignore[external-config-weakened] -- the fix of a CVE
```

```ini
; .npmrc
min-release-age=3 ; botica: ignore[external-config-weakened] -- the mirror lags behind the registry
```

pytest keeps the text after a value of its INI files (`pytest.ini`, `.pytest.ini`, `tox.ini`,
`setup.cfg`) as part of the value, so there the comment goes on a line of its own and covers the
file:

```ini
[pytest]
# botica: ignore-file[external-config-weakened] -- the suite of the payments client is flaky
xfail_strict = false
```

A comment without a reason is `invalid-suppression`, and one that silences nothing is
`unused-suppression`. For the whole project, `ignore = ["external-config-weakened"]` in the
configuration of botica, or a path in `extend-ignores` of `[rules.external-config-weakened]`.
