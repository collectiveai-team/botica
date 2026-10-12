# CES-71 · keep files small

**Code:** `CES-71` &nbsp;·&nbsp; **Slug:** `file-size-guard` &nbsp;·&nbsp; **Enforced by:**
`botica lint` (`warn` 400 / `error` 700), run by the `botica` prek hook &nbsp;·&nbsp; **Tracker:**
[#71](https://github.com/collectiveai-team/botica/issues/71)

## Directive

Keep source files small. The `file-size-guard` rule of `botica lint` **warns at 400 lines** and
**errors at 700 lines**. Split a module along a real seam before it crosses the hard limit.

## Why

- File length is a cheap proxy for too-many-responsibilities. A 700-line module almost always
  hides several modules that want to be separate.
- A persistently oversized file is a **design smell** — the fix is to extract a cohesive unit,
  not to raise the threshold.

## Thresholds

| Lines | Result | Meaning |
|---|---|---|
| `>= 400` | warning | large — plan a split |
| `>= 700` | error (fails the hook) | enormous — split now |

Lines are counted on the bytes of the file (each `\n`, plus a last line with no `\n`). Binary files
are skipped. The warning is shown, and does not fail the hook.

## Which files

Every file the walk reaches, in any language, not only Python. The walk honours `.gitignore`,
walks hidden directories such as `.github`, and skips the directories of tools, caches and
configuration of agents (`.git`, `.venv`, `.agents`, `.claude`…). Lockfiles, snapshots, minified
files and documentation (`.md`, `.rst`) are left out by the default `ignores` of the rule.

## When you hit the warning

1. Find the natural seam (a group of functions/classes that share state or a concern).
2. Extract it into a sibling module with a clear name (never a `utils.py` junk drawer).
3. Re-run `prek` to confirm the file is back under budget.

Raising the limit is not a remedy — flag the file as needing a refactor instead.

## Suppression

The finding is about the whole file and has no line, so a line comment cannot silence it. If a
single generated or vendored file legitimately must exceed the limit, put
`botica: ignore-file[file-size-guard]` in a comment of the file, or exclude it in the `botica`
configuration (`ignores`/`extend-ignores` of the rule, or `extend-exclude`), rather than removing
the hook.
