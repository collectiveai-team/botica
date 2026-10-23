---
id: CES-58
title: One modern lint stack
---

## Directive

Python code uses one toolchain: ruff (format and lint), pyrefly (types) and botica (the rules that
can be checked deterministically), all run through prek. The legacy stack is not reintroduced:
`black`, `isort`, `flake8` (and its plugins), `pylint`, `autopep8`, `yapf` and standalone
`pyupgrade`.

ruff selects the families of the section of `pyproject.toml` that the recipe `base-python` installs,
and among them `ASYNC` (a blocking call inside an `async def`), `A` (a name that hides a builtin or
a module of the standard library), `BLE` (an `except Exception` or `except BaseException` that
neither raises again nor logs the traceback), `SLOT` (a subclass of `str`, `tuple` or
`NamedTuple` without `__slots__`) and the codes `PLR0124` and `PLR0133` (a comparison of a name
with itself or of two constants).

Go code is formatted and linted by golangci-lint v2: `golangci-lint fmt` with the formatter
`gofumpt`, and `golangci-lint run` with the linters `standard` (errcheck, govet, ineffassign,
staticcheck, unused), `errorlint`, `wrapcheck`, `bodyclose`, `gosec`, `revive` with its rule
`package-naming`, and `godoclint` for the doc comments (CES-123), as `.golangci.yml` of the recipe
`base-go` lists them. A linter is added to that
list or removed from it by a change to this standard. `gocognit`, `gocyclo` and `funlen` are not
enabled: botica measures complexity (CES-110), and the size of a function is a judgment.

The type checker stays on. No setting or directive turns it off for a whole file, a project or a
build: `# pyrefly: ignore-errors`, `// @ts-nocheck`, `"strict": false` or an option of the strict
family set to `false` in `tsconfig.json`, and `typescript.ignoreBuildErrors` in `next.config`. A
type error that cannot be fixed yet is silenced on its line, with the reason.

The type checker checks every value. pyrefly runs with `preset = "strict"` and reports an `Any`
written in an annotation (`explicit-any = true`). No expression gives a value a type that nothing
checks: `typing.cast()` in Python; `as any`, `<any>x` or an assertion through `unknown` in
TypeScript, where Biome (`noExplicitAny`) also reports `any` in an annotation. A value of an
unknown type is narrowed with `isinstance`, a `TypeIs` function or a schema.

## Why

- ruff covers black, isort, flake8, pyupgrade and most of pylint in one fast tool with one config.
  Stacking the old tools on top means overlapping, conflicting passes and a config spread across
  five files.
- One toolchain means one configuration and one formatter.
- `E722` reports a bare `except:`, and `S110` and `S112` an `except` whose body is `pass` or
  `continue`. An `except Exception:` that logs a warning and returns passes the three; `BLE001`
  reports it. A `time.sleep` or an `open` inside an `async def` blocks the event loop, and only
  `ASYNC` reports it. An `assert x == x` passes whatever `x` is; `PLR0124` reports it.
- A checker that is off for a file reports nothing there, and the errors it would report stay in
  the code. An agent whose build fails on types can turn the checker off instead of fixing them.
- A parameter without an annotation has the type `Any`, and pyrefly checks nothing that flows from
  it, while it still reports 0 errors. A cast does the same for one value and leaves no comment
  that a reader or a tool can find.

## Replacements

| Legacy | Use |
|---|---|
| `black`, `yapf`, `autopep8` | `ruff format` |
| `isort` | `ruff` (import sorting, `I`) |
| `flake8` + plugins | `ruff` lint rules |
| `pyupgrade` | `ruff` (`UP`) |
| `pylint` | `ruff` + `pyrefly` |
| mypy (as the only checker) | `pyrefly` |

## When a legacy tool appears

A legacy tool in `pyproject.toml` or `prek.toml` is removed, and its intent is expressed through
this stack.

## Migration

A repository that installed the section of ruff before these families adds them to `select` in its
own pull request. `ruff check --select ASYNC,A,BLE,SLOT,PLR0124,PLR0133` lists what they report
first. A family that the repository does not want goes in `ignore`, where the rule `ruff-selection`
sees it. A finding
that stays, such as the `except Exception` of the loop of a worker, gets `# noqa: BLE001` with the
reason on its line.

A repository where the checker is off somewhere turns it back on in its own pull request:

1. Remove the directive or the setting, and run the checker (`pyrefly check`, `tsc --noEmit`).
2. Fix the errors it reports. An error that has to stay gets a suppression on its line with the
   reason: `# pyrefly: ignore[<code>]`, `// @ts-expect-error <reason>`.
3. When there are too many errors for one pull request, keep the file in the `extend-ignores` of
   `[rules.no-disabled-typecheck]` and remove it from there file by file.

A repository that moves to the strict preset runs `pyrefly check --preset strict` once to count
the errors, and sets the kinds with the most errors to `"warn"` in `[tool.pyrefly.errors]` until
they are fixed. A `baseline` file of pyrefly holds the existing errors only with
`baseline-matching-mode = "column-ordered"`: with the default mode, `"column"`, one entry of the
baseline hides every new error of the same kind at the same column of its file.
