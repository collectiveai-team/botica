## Directive

A suppression comment that names a rule botica does not know, or that is malformed, is a warning.
It applies to both channels: `# botica: ignore[...]` / `# botica: ignore-file[...]` and
`# noqa: BT...` (Python).

- An unknown rule ID is reported on that ID, unless it matches `external` in the configuration.
- A malformed directive is reported on the directive: `botica: ignore` without `[...]`, an empty
  `[]`, a `[` that is never closed, or a verb other than `ignore` and `ignore-file`.
- IDs are exact: a slug, a code or an alias. A family (`BTSTRUC`) or a prefix is unknown.
- A judgment rule is suppressed for a whole file, with a reason:
  `# botica: ignore-file[<slug>] -- <reason>`. A suppression of a judgment rule on a line, one
  written as `noqa`, and one with no reason after `--` are reported on the ID.
- A rule of project scope (`external-config`, `import-layers`, `settings-env-schema`) is silenced in the
  configuration, with `extend-ignores` in `[rules.<slug>]` or `ignore`. A comment that names one is reported on the
  ID.

A known rule that is not active in that file, or that has nothing to silence on that line, is not
invalid: it is `unused-suppression`.

## Why

A typo in a suppression silences nothing, so the finding it was meant to hide stays, and the
comment looks right. Ruff has the same check (`RUF102`).

## Suppression

Like any finding with a line, it is silenced by listing `invalid-suppression` (or `BT003`) in the
same comment:

```python
ATTEMPTS = 3  # botica: ignore[no-utilz, invalid-suppression]
```

or in an `ignore-file[...]` comment of the file:

```python
# botica: ignore-file[invalid-suppression]
```
