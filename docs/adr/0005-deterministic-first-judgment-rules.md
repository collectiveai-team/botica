# Deterministic first: judgment rules are rules of the catalog, for what botica cannot measure

**Status:** proposed

ADR 0003 lists `judgment` as a fourth kind of implementation, run by no part of `botica lint` and
catalogued for the judges and the site. ADR 0002 makes a standard with nothing to run a `judgment`
rule. Neither says where such a rule lives, what its folder holds, how it is numbered, or when a
standard is judgment and when it is a deterministic rule that waits for the repository to declare
something. The standards marked `[judgment]` in `AGENTS.md` (CES-5, 8, 16, 17, 30, 58, 64, 65, 66
and 77) include both cases: CES-58 is a check of the configuration of a repository, and CES-30 has
no check at all.

## Decision

1. **Botica is deterministic first.** A rule is `judgment` only when no deterministic check can
   express it, with or without configuration from the repository. Its `rule.toml` states why, in
   `why-not-deterministic`, and the catalog does not build without it.
2. **A rule that depends on the repository is deterministic, with a required option.** An option
   with no default (the layers of CES-5, the vocabulary of CES-16) makes the rule inactive until
   the project configures it, and `botica lint` reports an `info` that names the option. A model may
   help write that configuration once, through the `botica-rule-author` skill; it never runs at lint
   time.
3. **Judgment rules live in the same tree as the others,** `rules/<family>/<slug>/`, with
   `kind = "judgment"` on the implementation. Their code is the next free number of the family, as
   for any rule. A rule that becomes deterministic changes its implementation and keeps its folder,
   slug and code, so configurations, suppressions and links keep working.
4. **Each kind has a template, and the catalog enforces it.** `build.rs` fails on a folder that
   does not match the template of its kind, and `botica rules new --kind <kind>` writes a folder
   from it.

   | | Deterministic (`native`, `ast-grep`, `extension`) | `judgment` |
   |---|---|---|
   | `rule.toml` | `scope`, `needs`, `run-on`, `options` | `scope` (globs), `when` (`edit`, `turn` or `pull-request`), `question` (`boolean`, `choice` or `score`), `covered-by`, `why-not-deterministic` |
   | README | Directive, Why, Exceptions (optional), Do, Don't, Options (optional), Suppression | Directive, Why, Flag, Do not flag, Confirm, Do, Don't, Suppression |
   | Implementation | `python.yml`, `typescript.yml` or code in the core | none |
   | `fixtures/` | `valid/`, `invalid/` and `expected.json` with the range of each finding | `valid/`, `invalid/` and `expected.json` with the verdict of each file |

   `covered-by` lists the deterministic rules that check part of a judgment rule. The fixtures of a
   judgment rule are the set a judge is calibrated against.
5. **The catalog is the only source of a judgment rule.** Its text is the README of its folder. The
   index in `AGENTS.md`, the `engineering-rules` skill and the files of each judge are generated
   from the catalog by `botica rules export`, and none of them is edited by hand.
6. **Botica applies the suppressions of a judgment rule, by file.** A judgment rule judges a
   design, and its finding belongs to a file or to a whole change. Its suppressions are applied
   before the judge runs, in the files botica exports, and again on what the judge reports: a judge
   reports its findings as SARIF (`botica lint --judged <file>`), and botica filters them with the
   same engine, configuration and `overrides` as the findings of its own rules.

   | Level | Written as | Applied as |
   |---|---|---|
   | Repository | `ignore = ["<slug>"]` | the rule is left out of the export |
   | Path | `ignores` or `extend-ignores` of the rule | the paths are left out of the `scope` of the rule |
   | File | `botica: ignore-file[<slug>] -- <reason>` in a comment of the file | the file is left out of the `scope` of the rule |

   The reason after `--` is required. `invalid-suppression` reports a suppression of a judgment rule
   with no reason, one on a line (`botica: ignore[<slug>]`) and one written as `noqa`.
   `unused-suppression` does not evaluate the suppressions of a judgment rule, since `botica lint`
   never runs it. A finding about a whole change names no file, and only the repository and path
   levels silence it. The Do not flag section of the README lists what the rule leaves alone in
   every repository; a suppression is the exception of one repository.
7. **abide is the judge inside a coding session, and botica writes its rubric.** Botica runs no
   judge of its own. `botica rules export --format abide` writes `.abide/rubric.json` with
   `"manual": true`, which abide reads as written and never recompiles.

   | Field of the rubric | From |
   |---|---|
   | `id`, `text` | the slug and the Directive |
   | `source` | the line of the rule in the block of `AGENTS.md`, listed in `sources` |
   | `when` | `when` of the rule; a rule with `pull-request` is left out |
   | `scope` | the globs of the rule with the suppressions of decision 6 applied |
   | `check` | a judgment rule: `type = "model"`, its `question`, with criteria from Flag and Do not flag; a deterministic rule: `type = "lint"`, `overlaps = <slug>` |

   abide reads `scope` as globs joined by "or" (picomatch), and a negated glob in the list widens
   the match: `["src/**", "!src/legacy/**"]` matches `src/legacy/a.py` and `docs/x.md`. Botica
   writes positive globs only: a whole directory when nothing in it is excluded, and its files one by
   one when something is. abide judges only `model` rules with status `active`; a `lint` rule is
   counted and left to botica.

   - A hook of prek, `botica rules export --format abide --check`, fails when the rubric differs
     from the catalog, the configuration or the `ignore-file` comments of the repository. A new file
     in a directory with exclusions is outside the `scope` until the next export.
   - `botica rules test --judge abide` calibrates a rule: it runs `abide audit --json` on the
     fixtures and compares the verdicts with `expected.json`. `abide audit` judges the rules with
     `when = "edit"` only, in chunks of 150 lines, and reports each file with the verdicts of each
     rule (`byFile[].verdicts[]`: `ruleId`, `probability`, `band`).
   - In CI, `abide check --json` reports one section per phase. A section of the `edit` phase names
     one file; the section of the `turn` phase names every file of the change. Botica turns them into
     SARIF and filters them with `--judged`.
   - abide has two thresholds for all its rules (`act` and `flag`), so the severity of a judgment
     rule does not reach it. The diff goes to the endpoint of the key of the user (TypeSafe or the
     Vercel AI Gateway); botica sends nothing.
8. **A local model reaches abide through `TYPESAFE_AI_BASE_URL`.** With `TYPESAFE_AI_API_KEY` set,
   abide sends its calls to `<base URL>/systemone` instead of `https://api.typesafe.ai/v1`. The
   endpoint receives the model, the state (the diff, the file, the task) and one question per rule,
   and answers each question as a `choice` or a `score` with its probabilities, or as a boolean
   probability (`noul`). A local model needs an adapter that serves that API; Ollama and the
   OpenAI-compatible servers do not serve it.

## Not decided

- The formats of `botica rules export` other than `json` and `abide`. The order is `agents-md`,
  `skill`, `continue` and `abide`.
- Who serves the API of TypeSafe for a local model.
- The classification of each standard marked `[judgment]`, one pull request per standard.

## Considered options

- **A separate tree for judgment rules** (`judgments/<family>/<slug>/`, with a code prefix of its
  own). The kind is visible from the path. Rejected: a rule that becomes deterministic changes its
  folder and its code, and the old code becomes an alias.
- **The judge applies the suppressions,** told by the export to skip a line that carries the
  comment. Rejected: a model may ignore the instruction, and the suppression would stop being
  deterministic.
- **Suppressions on a line for judgment rules.** Rejected: abide answers one probability per rule
  and per file or change, with no line, so a suppression on a line has nothing to apply to.
- **A local proxy behind `TYPESAFE_AI_BASE_URL` that drops the questions of suppressed rules.**
  Rejected: it ties botica to the API of TypeSafe, it does not work with a key of the gateway, and
  it is a process that handles the credentials of the user.
- **A judge of botica's own.** Rejected: it needs a calibrated decision model, hooks for each agent
  and state per session, and abide has them.
- **A judgment rule for every standard that depends on the repository.** Rejected: a model would
  give a probable answer where the configuration of the repository gives an exact one.

## Consequences

- The kind table of ADR 0003 describes `judgment` as in this ADR.
- `rule.toml` gains `when`, `question`, `covered-by` and `why-not-deterministic` for judgment rules,
  and options with no default for deterministic rules.
- `botica lint` reads SARIF from a judge (`--judged`), and `invalid-suppression` checks the reason,
  the level and the channel of a suppression of a judgment rule.
- `botica rules export` gains `--format abide` and `--check`, and `botica rules test` gains
  `--judge abide`.
- `botica rules export --format json` writes `catalog.json`, which the other exports read: its
  `schema_version`, the families, and each rule in the order of its code with every field of its
  `rule.toml`, `added`, `since`, the path of its README and the sections of the README.
- The index of rules on the site needs a search with filters by kind, family, status and language,
  as Ruff's has.
- The README of each rule follows the template of its kind, and the section that lists what a rule
  leaves alone is called Exceptions.
