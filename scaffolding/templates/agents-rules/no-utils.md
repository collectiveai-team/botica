# CES-63 · no catch-all modules

**Code:** `CES-63` &nbsp;·&nbsp; **Slug:** `no-utils` &nbsp;·&nbsp; **Enforced by:** `botica lint`
(by file name), run by the `botica` prek hook &nbsp;·&nbsp; **Tracker:**
[#63](https://github.com/collectiveai-team/scaffolding/issues/63)

## Directive

Don't create catch-all modules. The `no-utils` rule of `botica lint` fails on any file named `utils.py`,
`helpers.py`, `aux.py`, `misc.py`, or `common.py` (anywhere except `tests/`). Name a module for
what it holds.

## Why

- `utils.py` is where cohesion goes to die: unrelated functions accumulate because there is no
  named home that says "this doesn't belong here". The grab-bag grows, gets imported everywhere,
  and becomes an undeletable dependency hub.
- A precise name (`text_wrapping.py`, `retry.py`, `iso_dates.py`) forces you to decide what the
  module is *about*, which keeps responsibilities sharp and imports honest.

## When you reach for `utils.py`

1. Name the concept the helper actually serves and make a module for it.
2. If it's one function used by one caller, inline it or co-locate it with that caller.
3. If several helpers cluster around a concept, that concept *is* the module name.

## Scope

The rule covers Python files only and skips anything under `tests/` (fixtures and test helpers are
exempt). No package parameter is needed — it keys purely on the basename.

## Suppression

The finding is about the file name and has no line, so a line comment cannot silence it. If a
single file legitimately must carry one of these names (rare — e.g. matching an external
convention), put `botica: ignore-file[no-utils]` in a comment of the file, or exclude it in the
`botica` configuration, rather than removing the hook.
