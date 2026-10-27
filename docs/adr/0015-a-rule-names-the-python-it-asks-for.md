# A rule names the versions of Python it asks for

**Status:** accepted

The standard of heavy imports (pull request #242) asks two things by the Python of the project.
Before 3.15, a heavy library is imported inside the function that uses it, and ruff `TID253`
reports it. From 3.15, it is imported at the top with `lazy import` (PEP 810), and ruff `PLC0415`
and the native rule `no-eager-heavy-imports` report it. ruff takes its target version from
`requires-python` and reports `lazy import` as invalid syntax before 3.15, so a project cannot run
both. The selection of a rule had no condition: `default`, `select` and the rules of a recipe made
it active in every project. #242 put each side in a recipe of its own that a project chooses by
hand, and `base-python` could select neither. botica read `requires-python` only to choose the
Python of the extensions (`crates/botica-cli/src/extension_python.rs`).

## Decision

1. **`python` in `rule.toml`.** A rule may name the versions it asks for in `python`: clauses
   joined by commas that all hold, each one of `>=`, `>`, `<=`, `<`, `==` or `!=` and a version of
   one to three numbers (`>=3.15`, `>=3.10, <3.15`). A catalog with any other text does not load.
   The field belongs to the rule and not to an implementation, since it decides whether the rule is
   active, and it applies to every kind of rule.
2. **The Python of the project is the lower bound of `requires-python`.** botica reads the
   `requires-python` of the `[project]` of `pyproject.toml` at the root and takes the highest of
   the lower bounds of its clauses: `>=`, `==`, `===` and `~=` give their version, without the
   `.*` or a pre-release, and `>V` the first release after `V` (`>3.14` gives 3.14.1, as PEP 440
   allows 3.14.1). `<`, `<=` and `!=` set no lower bound. The code of the project has to run on the
   lowest version it allows, so that version decides which syntax a rule may ask for.
   `.python-version` and `.venv` name the Python of one machine, and do not count.
3. **No lower bound counts as older than every range.** A project with no `pyproject.toml`, no
   `requires-python`, a `requires-python` without a lower bound (`<4`) or one botica cannot read
   has no version: a rule whose `python` sets a lower bound (`>=3.15`, `==3.14`) is not active
   there, and one with only upper bounds and exclusions (`<3.15`, `!=3.13`) is. botica assumes no
   version, so `requires-python` is the one place that names the Python of the project, for
   botica and for ruff. ruff 0.17.1 does the same: without `target-version` or `requires-python`
   it applies its default, `py311` ([settings](https://docs.astral.sh/ruff/settings/#target-version)),
   so `UP045` reports `Optional[int]`; with `target-version = "py311"`, the setting of this
   repository, it reports `lazy import json` as `invalid-syntax: Cannot use lazy import statement
   on Python 3.11 (syntax was added in Python 3.15)`. With no version at all, `ruff check
   --show-settings` gives `linter.unresolved_target_version = none` and ruff reports no syntax
   error for `lazy import`. A project that writes `lazy import` sets `requires-python = ">=3.15"`,
   which CES-77 suggests for a new repository and which both tools then read.
4. **A selected rule outside the range is not active.** After the selection and the replacements,
   and before `--scope`, each active rule whose `python` does not hold the version of the project
   becomes not active, with the reason `requires Python <range>, and <what the project asks>`.
   `botica rules --all`, `botica rules --json` and `botica explain` show it; a rule that the
   configuration ignores or switches off keeps that reason.
5. **The commands that read the activity follow.** `botica lint` runs no part of such a rule:
   `external-config` asks nothing for its fragment, and the block of `AGENTS.md` and the rubric of
   abide leave it out. `botica recipe check` writes `ok, not active: <reason>` for it and checks
   no fragment of it, so a recipe can list both sides and pass in every project.

## Considered options

- **`python` on an implementation.** Rejected: whether a rule is active is decided for the rule,
  and the reasons of `botica rules` are of a rule. The two sides of #242 are three rules.
- **A project without `requires-python` is unknown, and no rule with `python` runs.** Rejected: a
  repository that names no version would run neither side of #242, and `base-python` would ask it
  for nothing.
- **No lower bound counts as the default of CES-77.** Rejected: CES-77 suggests a version for a
  new repository and does not impose it. botica would run the rules for 3.15 in a project that
  ruff checks as 3.11, and `requires-python` would no longer be the one source of the version.
- **The Python that `uv` would choose, as for the extensions.** Rejected: `.python-version` names
  the interpreter of development, often newer than the lowest version the package supports, and a
  rule for 3.15 would ask for syntax that the users on an older Python cannot import.
- **`python` on the steps of a recipe.** Left out: no recipe needs it yet. `base-python` writes the
  tables of ruff with `[[files]]` and `[[sections]]`, and a TOML table cannot appear twice, so
  #242 needs only the selection of its rules.

## Consequences

- `botica-catalog` parses the range and the lower bound with code of its own (`python.rs`); a
  `requires-python` with an epoch or a local version is read by its release only.
- botica reads `pyproject.toml` at the root of the project on each run. A nested `pyproject.toml`
  of a workspace does not count, as for `external-config` (ADR 0014).
- `[[files]]` and `[[sections]]` of a recipe are written in every project. A fragment that depends
  on the version goes into the external rule with `python`, which `external-config` checks only
  where the rule is active.
- The fixtures of a rule with `python` hold a `pyproject.toml` with a `requires-python` inside the
  range, since `botica rules conform` lints each side as a project of its own.
- The cache of `botica lint` keys on the active rules, so a change of `requires-python` that
  changes them runs every rule again.
