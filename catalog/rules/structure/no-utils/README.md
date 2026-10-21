## Directive

Don't create catch-all modules. A file named `utils.py`, `helpers.py`, `aux.py`, `misc.py` or
`common.py` is an error, anywhere except under `tests/`. Name a module for what it holds.

## Why

- Unrelated functions collect in `utils.py` because its name rejects none of them. Many modules
  import it, so it cannot be deleted.
- A precise name (`text_wrapping.py`, `retry.py`, `iso_dates.py`) states what the module holds.

## Do

1. Name the concept the helper serves and make a module for it.
2. If it is one function used by one caller, inline it or keep it next to that caller.
3. If several helpers cluster around a concept, that concept is the module name.

```text
myapp/
  retry.py           # retry with backoff
  iso_dates.py       # parse and format ISO 8601 dates
  text_wrapping.py
```

## Don't

```text
myapp/
  utils.py           # retry, date parsing and text wrapping
```

## Suppression

The finding belongs to the file name. An `ignore-file` comment anywhere in the file silences it:

```python
# botica: ignore-file[no-utils]
```

`extend-ignores` in the configuration of the rule leaves out the files that match a glob, such as a
catch-all module that the repository already has:

```toml
# botica.toml
[rules.no-utils]
extend-ignores = ["src/legacy/utils.py"]
```
