## Directive

Don't create catch-all modules. A module named `utils`, `helpers`, `aux`, `misc` or `common` is an
error, anywhere except under `tests/`. It applies to the files of Python (`.py`), TypeScript (`.ts`,
`.tsx`, `.mts`, `.cts`), JavaScript (`.js`, `.jsx`, `.mjs`, `.cjs`), Go (`.go`) and Rust (`.rs`). In
TypeScript and JavaScript a module named `util`, `shared` or `lib` is a warning. Name a module for
what it holds.

## Why

- Unrelated functions collect in `utils.py` because its name rejects none of them. Many modules
  import it, so it cannot be deleted.
- A precise name (`text_wrapping.py`, `retry.ts`, `iso_dates.rs`) states what the module holds.

## Exceptions

- The rule reads file names and never folder names. A folder is a namespace and names its
  contents, so `utils/text.py`, `utils/index.ts`, `utils/__init__.py` and `utils/mod.rs` pass. In
  Rust `utils.rs` is a file and is reported.
- A test file has the name of what it tests (`utils.test.ts`, `utils_test.go`) and passes. `lib.rs`,
  the root of a Rust crate, passes.
- In Go other code imports the package, which is the folder, and not the file. `common.go` is
  reported; a package named `utils` or `common` is the rule `package-naming` of revive, which the
  `.golangci.yml` of the recipe `base-go` turns on.

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
web/src/
  helpers.ts
  shared.ts          # warning
internal/orders/
  common.go
src/
  misc.rs
```

## Suppression

The finding belongs to the file name. An `ignore-file` comment anywhere in the file silences it:

```python
# botica: ignore-file[no-utils]
```

```typescript
// botica: ignore-file[no-utils]
```

A `severity` in the configuration of the rule replaces both of its own: with `severity =
"warning"`, `utils.py` is a warning too.

`extend-ignores` in the configuration of the rule leaves out the files that match a glob, such as a
catch-all module that the repository already has:

```toml
# botica.toml
[rules.no-utils]
extend-ignores = ["src/legacy/utils.py", "web/src/helpers.ts"]
```
