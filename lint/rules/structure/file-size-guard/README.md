# file-size-guard · `BTSTRUC001`

**Standard:** CES-71 (documentation only) · **Family:** structure · **Languages:** all

## Directive

Keep source files small. A file with **400 lines or more** gets a warning; **700 or more** is an
error. Split a module along a real seam before it crosses the hard limit.

## Why

- File length is a cheap proxy for too many responsibilities. A 700-line module almost always
  hides several modules that want to be separate.
- A persistently oversized file is a design smell. The fix is to extract a cohesive unit, not to
  raise the threshold.

## Options

| Option | Default | Meaning |
|---|---|---|
| `warn` | 400 | lines at which the warning starts |
| `max` | 700 | lines at which it becomes an error |

Lines are counted on the bytes of the file: each `\n`, plus a last line that has no `\n`. A lone
`\r` is not a line break. Binary files (a NUL in the first 8 KB) are skipped.

## Which files

Every file the walk reaches. The walk honours `.gitignore`, walks hidden directories such as
`.github`, and skips the directories of tools, caches and configuration of agents (`.git`,
`.venv`, `.mypy_cache`, `.agents`, `.claude`…).
`extend-include` brings back a path they leave out, and `extend-exclude` wins over it.

Of those, the generated or documentation files in the rule's default `ignores` are left out
(lockfiles, snapshots, translations, minified files, maps, SVG, `.rst`/`.md`/`.mdx` and JSON under
`fixtures/`). Replace them with `ignores` or extend them with `extend-ignores` in the project
configuration.

## Suppression

The finding is about the whole file and has no line, so it cannot be silenced with a line comment.
Put `botica: ignore-file[file-size-guard]` in a comment of the file, or exclude the file in the
configuration.
