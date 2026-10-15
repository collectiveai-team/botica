## Directive

Don't create catch-all modules. A file named `utils.py`, `helpers.py`, `aux.py`, `misc.py` or
`common.py` is an error, anywhere except under `tests/`. Name a module for what it holds.

## Why

- `utils.py` is where cohesion goes to die: unrelated functions accumulate because there is no
  named home that says "this doesn't belong here". The grab-bag grows, gets imported everywhere
  and becomes an undeletable dependency hub.
- A precise name (`text_wrapping.py`, `retry.py`, `iso_dates.py`) forces you to decide what the
  module is about, which keeps responsibilities sharp.

## When you reach for `utils.py`

1. Name the concept the helper serves and make a module for it.
2. If it is one function used by one caller, inline it or keep it next to that caller.
3. If several helpers cluster around a concept, that concept is the module name.

## Suppression

The finding is about the file name and has no line, so it cannot be silenced with a line comment.
Put `botica: ignore-file[no-utils]` in a comment of the file, or exclude the file in the
configuration.
