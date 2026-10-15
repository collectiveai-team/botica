## Directive

A suppression comment that names a rule botica does not know, or that is malformed, is a warning.
It applies to both channels: `# botica: ignore[...]` / `# botica: ignore-file[...]` and
`# noqa: BT...` (Python).

- An unknown rule ID is reported on that ID, unless it matches `external` in the configuration.
- A malformed directive is reported on the directive: `botica: ignore` without `[...]`, an empty
  `[]`, a `[` that is never closed, or a verb other than `ignore` and `ignore-file`.
- IDs are exact: a slug, a code or an alias. A family (`BTSTRUC`) or a prefix is unknown.

A known rule that is not active in that file, or that has nothing to silence on that line, is not
invalid: it is `unused-suppression`.

## Why

A typo in a suppression silences nothing, so the finding it was meant to hide stays, and the
comment looks right. Ruff has the same check (`RUF102`).

## Suppression

Like any finding with a line: list `invalid-suppression` (or `BT003`) in the same comment, or in an
`ignore-file[...]` of the file.
