## Directive

Keep source files small. A file with 400 lines or more gets a warning; 700 or more is an
error. Split a module along a real seam before it crosses the hard limit.

## Why

- The length of a file is a cheap measure of how many responsibilities its module holds. A
  700-line module holds several units that can each be a module.
- An oversized file is fixed by extracting a cohesive unit. Raising the threshold leaves the
  module as it was.

## Exceptions

The rule reads every file the walk reaches. The walk honours `.gitignore`, walks hidden directories
such as `.github`, and skips the directories of tools, caches and configuration of agents (`.git`,
`.venv`, `.mypy_cache`, `.agents`, `.claude`…). `extend-include` brings back a path they leave out,
and `extend-exclude` wins over it.

Of those, the generated or documentation files in the rule's default `ignores` are left out
(lockfiles, snapshots, translations, minified files, maps, SVG, `.rst`/`.md`/`.mdx`, JSON under
`fixtures/` and the rubric of abide, `.abide/rubric.json`). Replace them with `ignores` or extend them with `extend-ignores` in the project
configuration. Binary files (a NUL in the first 8 KB) are skipped.

## Do

Split a module along the seams of what it does:

```text
billing/
  invoices.py        # 310 lines: building an invoice
  payments.py        # 280 lines: charging a card
  refunds.py         # 190 lines: refunding a payment
```

## Don't

```text
billing.py           # 780 lines: invoices, payments and refunds
```

```toml
# botica.toml: a higher limit hides the module that wants to be split
[rules.file-size-guard]
max = 1500
```

## Options

| Option | Default | Meaning |
|---|---|---|
| `warn` | 400 | lines at which the warning starts |
| `max` | 700 | lines at which it becomes an error |

Lines are counted on the bytes of the file: each `\n`, plus a last line that has no `\n`. A lone
`\r` is not a line break.

## Suppression

The finding belongs to the whole file. An `ignore-file` comment anywhere in the file silences it,
written in the comment syntax of the file: `#` in Python, shell, YAML and TOML, `//` in JavaScript
and TypeScript, `<!-- -->` in HTML and Markdown.

### Python

```python
# botica: ignore-file[file-size-guard]
```

### TypeScript

```ts
// botica: ignore-file[file-size-guard]
```

### Configuration

`extend-ignores` leaves out the files that match a glob:

```toml
# botica.toml
[rules.file-size-guard]
extend-ignores = ["src/generated/**"]
```
