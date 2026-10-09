# unused-suppression · `BT002`

**Family:** none (rule of the linter itself) · **Languages:** all

## Directive

A suppression comment that silences nothing is a warning. It applies to both channels:
`# botica: ignore[...]` and `# noqa: BT...`.

## Why

A stale suppression hides nothing today and will hide a real finding tomorrow. Ruff has the same
check (`RUF100`).

## Suppression

This rule can silence itself, as `RUF100` does: list `unused-suppression` (or `BT002`) in the same
comment.
