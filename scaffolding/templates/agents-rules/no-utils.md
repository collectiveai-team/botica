# CES-63 · no catch-all modules

**Code:** `CES-63` &nbsp;·&nbsp; **Slug:** `no-utils` &nbsp;·&nbsp; **Enforced by:** prek hook
(filename glob) &nbsp;·&nbsp; **Tracker:**
[#63](https://github.com/collectiveai-team/botica/issues/63)

## Directive

Don't create catch-all modules. The `no-utils` prek hook fails on any file named `utils.py`,
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

The hook excludes `tests/` (fixtures and test helpers are exempt). No package parameter is
needed — it keys purely on the basename.

## Suppression

`no-utils` is a prek hook, so there is no per-line ast-grep ignore. If a single file legitimately
must carry one of these names (rare — e.g. matching an external convention), exclude it via the
hook's `exclude` pattern in `prek.toml` rather than disabling the guard.
