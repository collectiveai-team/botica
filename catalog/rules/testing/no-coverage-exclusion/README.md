## Directive

The configuration of coverage.py leaves no code out of its report. The rule reads
`[tool.coverage.run]` and `[tool.coverage.report]` of `pyproject.toml`, `[run]` and `[report]` of
`.coveragerc`, and `[coverage:run]` and `[coverage:report]` of `setup.cfg` and `.coveragerc`, and
reports:

- each entry of `omit`, `exclude_lines` and `exclude_also`;
- a `fail_under` lower than the one before the change. Only `botica lint --diff`, `--staged` and
  `--diff-file` know the value before the change; without them a `fail_under` is not reported.

With a diff, only the entries on the lines that the change adds are reported. A warning by default.

## Why

An entry of `omit` or `exclude_lines` raises the percentage of coverage and tests nothing: the
files and lines it names have no test, and a bug in them stays unseen. A lower `fail_under` lets a
change pass with less of the code tested than before. The gap is a missing test, dead code, or a
seam to refactor, and each has a fix that is not the setting.

## Do

```toml
[tool.coverage.report]
fail_under = 90
show_missing = true
omit = ["src/orders/_version.py"]  # botica: ignore[no-coverage-exclusion] -- written by setuptools-scm
```

## Don't

```toml
[tool.coverage.run]
omit = ["src/orders/legacy/*"]

[tool.coverage.report]
exclude_also = ["raise NotImplementedError"]
```

## Suppression

In `pyproject.toml`, `# botica: ignore[no-coverage-exclusion] -- <reason>` on the line of the
entry, for code that cannot run under test, such as generated code. configparser reads a `#` after
a value as part of the value, so `setup.cfg` and `.coveragerc` take
`# botica: ignore-file[no-coverage-exclusion] -- <reason>` on a line of its own, or the file in the
`extend-ignores` of `[rules.no-coverage-exclusion]`.
