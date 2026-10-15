## Directive

A suppression comment that silences nothing is a warning. It applies to both channels:
`# botica: ignore[...]` and `# noqa: BT...`. A suppression of a judgment rule is never unused:
`botica lint` never runs a judgment rule, and a judge reads the suppression.

## Why

A stale suppression hides nothing today and will hide a real finding tomorrow. Ruff has the same
check (`RUF100`).

## Suppression

The rule can silence itself, as `RUF100` does: list `unused-suppression` (or `BT002`) in the same
comment.

```python
x = 1  # botica: ignore[no-utils, unused-suppression]
```
