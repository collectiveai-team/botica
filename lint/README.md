# botica lint

A deterministic linter for the house engineering standards. It never calls a model or the
network, reads your files once, and runs its rules in parallel. The design is in
[ADR 0003](../docs/adr/0003-botica-lint-architecture.md). The catalog under `rules/` is the only
documentation of the rules it measures: each rule has a README, and `botica rules` lists them. The
CES are catalogued in [`docs/engineering-standards.md`](../docs/engineering-standards.md).

## Install

- **From a wheel**, published on PyPI since 0.7.0 (see
  [`docs/botica-release.md`](../docs/botica-release.md)): `uv tool install botica` or
  `uvx botica lint`. The wheel holds only the binary, so it needs no Python at run time.
- **From source**: `cargo build --release -p botica-cli` in this directory; the binary is
  `target/release/botica`.

## Use

```bash
botica lint                    # the current directory
botica lint src tests/a.py     # directories to walk, or files (as prek passes them)
botica lint --select file-size-guard,no-utils
botica lint --output-format json   # text (default), json, sarif or github
botica rules                   # which rules are active, and why (--all adds the inactive ones)
botica explain no-notebooks    # what a rule is and how it is configured here
```

Exit code: `0` clean (a warning does not fail the run), `1` a finding of severity error, `2`
the run itself failed (it wins over `1`). `--exit-zero` forgives only the findings.

## Rules

| Code | Rule | Standard | Looks at |
|---|---|---|---|
| `BTSTRUC001` | [`file-size-guard`](rules/structure/file-size-guard/README.md) | CES-71 | every file: warns at 400 lines, errors at 700 |
| `BTSTRUC002` | [`no-utils`](rules/structure/no-utils/README.md) | CES-63 | Python: `utils.py`, `helpers.py`, … |
| `BTSTRUC003` | [`no-artifacts-in-package`](rules/structure/no-artifacts-in-package/README.md) | CES-32 | data, models and reports inside the code |
| `BTSTRUC004` | [`no-notebooks`](rules/structure/no-notebooks/README.md) | CES-32 | notebooks inside the code |
| `BTSTRUC005` | [`no-tests-in-package`](rules/structure/no-tests-in-package/README.md) | none yet | Python: tests inside the package |
| `BTARCH001` | [`arch-database-package`](rules/architecture/arch-database-package/README.md) | CES-18 | Python: tables and engines outside a `database` package |
| `BTARCH002` | [`settings-module`](rules/architecture/settings-module/README.md) | CES-76 | Python: `os.getenv` and `os.environ` outside the settings module |
| `BTDM001` | [`api-schemas-extra-forbid`](rules/data-models/api-schemas-extra-forbid/README.md) | CES-4 | Python: API schemas without `extra="forbid"` |
| `BTDM002` | [`no-dict-alias`](rules/data-models/no-dict-alias/README.md) | CES-79 | Python: a type alias for a raw `dict` |
| `BTDM003` | [`no-dict-call-return`](rules/data-models/no-dict-call-return/README.md) | CES-79 | Python: `return dict(...)` |
| `BTDM004` | [`no-dict-literal-return`](rules/data-models/no-dict-literal-return/README.md) | CES-79 | Python: returning a dict literal |
| `BTDM005` | [`no-dict-return-annotation`](rules/data-models/no-dict-return-annotation/README.md) | CES-79 | Python: a function annotated to return `dict` |
| `BTLOG001` | [`log-get-logger`](rules/logging/log-get-logger/README.md) | CES-45 | Python: `logging.getLogger` called directly |
| `BTLOG002` | [`log-no-print`](rules/logging/log-no-print/README.md) | CES-46 | Python: `print()` in library code |
| `BTLOG003` | [`no-console-log`](rules/logging/no-console-log/README.md) | none yet | TypeScript: `console.log` in library code |
| `BTPROC001` | [`cli-typed-framework`](rules/process/cli-typed-framework/README.md) | CES-67 | Python: `argparse`, `click` and `sys.argv` |
| `BT001`–`BT003` | `io-error`, `unused-suppression`, `invalid-suppression` | | the run and the suppressions |

The rules written in ast-grep's YAML run too, unless you pass `--no-astgrep`. Each rule has a
README with its directive, its reasons and how to silence it.

## Which files

The walk honours `.gitignore` and enters hidden directories (`.github/`…) except the directories
of tools, caches and agent configuration (`.git`, `.venv`, `.mypy_cache`, `.botica`, `.agents`,
`.claude`…; the list is `TOOL_DIRS` in `crates/botica-core/src/walk/mod.rs`). A path named on the
command line is linted even if it is on that list or excluded, unless you pass `--force-exclude`,
which leaves out what `extend-exclude` names and what sits in a directory of a tool (unless
`extend-include` asks for it). The `botica` hook of the mirror passes it, because prek names every
file.

- `extend-exclude = ["vendor/"]` skips more (gitignore globs); it wins over everything.
- `extend-include = ["generated/api.py"]` brings back what `.gitignore` or `TOOL_DIRS` leave out.

## Configuration

`botica.toml`, or `[tool.botica]` in `pyproject.toml`, found by walking up from the working
directory. Defining both is an error, and so is an unknown key or rule. The JSON Schema is
[`schema/botica.schema.json`](schema/botica.schema.json).

```toml
# botica.toml
extend-exclude = ["vendor/"]
ignore = ["no-tests-in-package"]

[rules.file-size-guard]
warn = 500
max = 900
```

Keys: `select`, `extend-select`, `ignore`, `preview`, `languages`, `extend-exclude`,
`extend-include`, `code-dirs`, `rules` (settings per rule), `overrides`, `external`, `replace`,
`rule-dirs`, `catalogs`, `compat-ast-grep-ignore`. Any of them can be given for one run with
`--config 'key = value'`.

`code-dirs` says where the code of the project lives (`python`, `js`, `rust`, `go`, `jvm`), for
the layout rules. Without it they are detected from `pyproject.toml`, `package.json`,
`Cargo.toml`, `go.mod` and the like; `botica explain no-notebooks` lists what was found.

## Silence a finding

```python
x = compute()  # botica: ignore[some-rule]
# botica: ignore-file[file-size-guard]    <- anywhere in the file
y = 1  # noqa: BTSTRUC002 -- reason
```

A finding about a whole file (size, name, layout) has no line: use `ignore-file`, the rule's
`ignores`/`extend-ignores`, or `extend-exclude`. With ruff, put `lint.external = ["BT"]` in its
configuration so it does not report `RUF102` for `BT` codes (the scaffolding template does).

While a repository migrates from ast-grep, `compat-ast-grep-ignore = true` makes botica honour
`# ast-grep-ignore: <rule>` too, on the line of a finding and on the line before it. It is off by
default, and the README of a rule does not repeat it.

## prek

```toml
# prek.toml: the mirror installs the wheel of the same version from PyPI
[[repos]]
repo = "https://github.com/collectiveai-team/botica-pre-commit"
rev = "v0.8.0"

[[repos.hooks]]
id = "botica"
args = ["--select", "<the rules of prek-generic.toml>"]
```

The mirror's hook is `verbose`: prek hides the output of a passing hook, and a size warning does
not fail it ([`distribution/botica-pre-commit`](distribution/botica-pre-commit/README.md)). The
scaffolder ships this hook, with the rules it selects, in `scaffolding/templates/prek-generic.toml`.

## Result cache

`--cache` keeps per-file results in `.botica/cache` under the project root; `--cache-dir <dir>`
chooses another place. It is off by default and meant for CI and `--all-files`. The key is the file
path and bytes; the whole cache is discarded when the version, the configuration or the rules
change. A directory botica creates ignores itself in git; one that already existed is left alone,
so add it to your `.gitignore`. A cache that cannot be written (a read-only checkout) is a warning
on stderr, not a failure: the findings and the exit code are the same, only the next run is slower.

## Develop

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --no-fail-fast
```

CI (`.github/workflows/ci.yml`, which checks the whole workspace) also sets `BOTICA_REQUIRE_RUFF=1`, so the catalog test
cannot skip itself for lack of `ruff`, and `BOTICA_REQUIRE_READ_ERRORS=1`, so the permission tests
cannot skip themselves when the runner is root. Set them locally to run as CI does. `bench/` has
the parity and timing scripts (`parity.py`, `hook_parity.py`, `run.sh`).
