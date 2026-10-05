# CES-71 · keep files small

**Code:** `CES-71` &nbsp;·&nbsp; **Slug:** `file-size-guard` &nbsp;·&nbsp; **Enforced by:** prek
hook (`warn` 400 / `error` 700) &nbsp;·&nbsp; **Tracker:**
[#71](https://github.com/collectiveai-team/botica/issues/71)

## Directive

Keep source files small. The `file-size-guard` prek hook **warns at 400 lines** and **errors at
700 lines**. Split a module along a real seam before it crosses the hard limit.

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

## When you hit the warning

1. Find the natural seam (a group of functions/classes that share state or a concern).
2. Extract it into a sibling module with a clear name (never a `utils.py` junk drawer).
3. Re-run `prek` to confirm the file is back under budget.

Raising the limit is not a remedy — flag the file as needing a refactor instead.

## Suppression

`file-size-guard` is a prek hook, so there is no per-line ast-grep ignore. If a single generated
or vendored file legitimately must exceed the limit, exclude it via the hook's `exclude` pattern
in `prek.toml` rather than disabling the guard globally.
