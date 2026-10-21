# Deterministic first: judgment rules are rules of the catalog, for what botica cannot measure

**Status:** accepted

**Amended 2026-10-20:** `AGENTS.md` lists no standard. The recipe `base` lists the judgment rules
under `judgment` and asks for the exports `agents-md` and `abide`, and `botica recipe install`
runs them after its sections: the section `## Botica rules` brings the markers of the block, the
export fills the block with one line per active judgment rule, and the rubric names those lines.
A configuration that `install` writes gets `preview = true` when a rule of the recipes is in
preview. The hand-written index of standards that the scaffolder installed is removed.

Amended 2026-10-20, again: ADR 0008 removed the skill `engineering-rules`, so decision 5 generates
the block of `AGENTS.md` and the files of each judge only. The classification that "Not decided"
lists is done: the standards marked `[judgment]` have judgment rules, `import-layers` or
`no-disabled-typecheck`. An `extension` rule of the catalog of a project runs in `botica lint`
(docs/botica.md, "Catalogs of a project").

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
4. **Each kind has a template, and the catalog enforces it.** The catalog does not load a folder
   that does not match the template of its kind, so the tests of `botica-catalog` and every command
   of botica fail on it, and `botica rules new --kind <kind>` writes a folder from it. The sections
   of a README are a closed list in a fixed order: a section that the template does not name, one
   out of its order, a missing required one and text before the first are errors. A rule of botica
   itself (`rules/botica/`) has the sections Directive, Why and Suppression.

   | | Deterministic (`native`, `ast-grep`, `extension`) | `judgment` |
   |---|---|---|
   | `rule.toml` | `scope`, `needs`, `run-on`, `options` | `files` and `ignores` (globs); on the implementation, `when` (`edit`, `turn` or `pull-request`), `question` (`boolean`, `choice` or `score`), `covered-by`, `why-not-deterministic`, `act-at` |
   | README | Directive, Why, Exceptions (optional), Do, Don't, Options (optional), Suppression | Directive, Why, Flag, Do not flag, Confirm, Do, Don't, Suppression |
   | Implementation | `python.yml`, `typescript.yml` or code in the core | none |
   | `fixtures/` | `valid/`, `invalid/` and `expected.json` with the range of each finding | `valid/`, `invalid/` and `expected.json` with the verdict of each file |

   `covered-by` lists the deterministic rules that check part of a judgment rule. A judgment rule
   whose `covered-by` names a rule active in the project is left out of the block of `AGENTS.md`
   and of the rubric, and `botica rules` says so on its line. `botica rules new <family>/<slug>
   --from <judgment rule>` writes a deterministic rule with the summary, the standard and the text
   of the judgment rule, and adds it to the `covered-by` of the judgment rule; `botica rules eval
   <rule> [paths]` runs that one rule over real code, whatever the configuration selects, for a
   review of its false positives. The fixtures of a
   judgment rule are the set a judge is calibrated against. A judgment rule has one implementation,
   of kind `judgment`, and the fields of a judge live on it: a rule that becomes deterministic
   replaces that implementation and keeps the root of its `rule.toml`. A deterministic rule
   requires `scope`, `needs` and `run-on`; a judgment rule rejects them and `prefilter`, and another
   kind of implementation rejects the fields of a judge.
5. **The catalog is the only source of a judgment rule.** Its text is the README of its folder. The
   index in `AGENTS.md`, the `engineering-rules` skill and the files of each judge are generated
   from the catalog by `botica rules export`, and none of them is edited by hand.
6. **Botica applies the suppressions of a judgment rule, by file.** A judgment rule judges a
   design, and its finding belongs to a file or to a whole change. Its suppressions are applied
   before the judge runs, in the files botica exports, and again on what the judge reports: a judge
   reports its findings as SARIF or as the report of `abide check --json`
   (`botica lint --judged <file>`), and botica filters them with the
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
   judge of its own. `botica rules export --format abide` writes `.abide/rubric.json`, the one
   path where abide reads a rubric (`--output` names another), with `"manual": true`, which abide
   reads as written and never recompiles. abide stays a tool apart from botica, and three JSON
   documents join them: the rubric, the report of `abide audit --json` and the SARIF that
   `--judged` reads. The rubric holds the
   active judgment rules of the project and no deterministic rule: abide never runs a `lint`
   rule, and `botica lint` runs them.

   | Field of the rubric | From |
   |---|---|
   | `id`, `text` | the slug and the Directive, which the catalog limits to 600 characters for a judgment rule |
   | `source` | the line of the rule in the block of the file of `--agents-md` (`AGENTS.md` by default), listed in `sources` |
   | `when` | `when` of the rule; a rule with `pull-request` is left out |
   | `scope` | the files of the project that the rule judges, with the suppressions of decision 6 applied; a rule with no such file is left out |
   | `check` | `type = "model"` and its `question` as `rule.toml` writes it |

   The question of a judgment rule lives in `rule.toml`, in the form abide reads
   (`@coldtea/abide-schema` 0.0.5): a table `[implementations.question]` with `type` (`boolean`,
   `choice` or `score`), `instructions`, and `criteria`: `{ true, false }` for a boolean, a map of
   options with `violating` for a choice, a list from compliant to violating with `violatingFrom`
   for a score. abide reads the probability of `true` of a boolean as the probability that the rule
   is broken, so a boolean question asks whether the file breaks the rule, and `true` describes the
   violation. A question asked the other way round judges each compliant file as broken; the
   fixtures of the rule show it when `botica rules test --judge abide` runs.

   The block of `AGENTS.md` is written by `botica rules export --format agents-md --output
   AGENTS.md`, which replaces the text between `<!-- botica:rules -->` and `<!-- /botica:rules -->`
   and leaves the rest of the file as it is; it fails when the markers are missing, and `--check`
   fails when the block differs. The block lists the active judgment rules of the project and no
   deterministic rule, since an agent has no choice about a rule that `botica lint` runs. It has a
   heading `### <family>` per family, and under it one line per rule in the order of their codes:
   ``- <code> `<slug>`: <first sentence of the Directive> → <URL of its page>``. The URL starts with
   `--base-url`, the standards site by default. The rubric is exported after it, so `source` names
   the line of each rule.

   `--check` of the rubric compares `sources` and `rules` only: it ignores `compiledAt`,
   `compiledBy`, the `sha` of each source and a `status = "active"` written out, which
   `abide rubric validate` adds when it rewrites the file. The export leaves a rubric with the same
   `sources` and `rules` as it is, `compiledAt` included.

   abide reads `scope` as globs joined by "or" (picomatch), and a negated glob in the list widens
   the match: `["src/**", "!src/legacy/**"]` matches `src/legacy/a.py` and `docs/x.md`. Botica
   writes positive globs only. It walks the files of the project from its root, as `botica lint`
   walks them, and keeps those in the languages of the rule where `botica lint --judged` would
   keep a finding of the rule. A directory whose files of those languages are all kept is one glob
   per extension of the languages (`src/**/*.py`, `src/**/*.pyi`), or `src/**` for a rule of every
   language; any other directory lists its kept files one by one. A file that `extend-exclude`
   leaves out of the walk counts as a file of its directory that is not kept, so the glob of the
   directory does not cover it. abide judges only `model` rules
   with status `active`.

   - The hook `botica-rubric` of the mirror, `botica rules export --format abide --check`, which
     the recipe `base` installs, fails when the rubric differs
     from the catalog, the configuration or the `ignore-file` comments of the repository. A new file
     in a directory with exclusions is outside the `scope` until the next export.
   - `botica rules test --judge abide` calibrates a rule: it runs `abide audit --json` on the
     fixtures and compares the verdicts with `expected.json`. `abide audit` judges the rules with
     `when = "edit"` only, in chunks of 150 lines, and reports each file with the verdicts of each
     rule (`byFile[].verdicts[]`: `ruleId`, `probability`, `band`).
     The `expected.json` of a judgment rule is `{"verdicts": [{"file", "band", "min", "max"}]}`:
     one verdict for each file of `valid/` and `invalid/`, `band` `clear` for a valid file and
     `act` or `flag` for an invalid one, and `min` and `max`, optional, the range of the
     probability. A fixture matches when abide gives it that band and a probability in the range.
     `rules test` checks `expected.json` before it runs abide, and runs it in
     `.botica/judge/<slug>`, a git repository with the fixtures and a rubric with the rule alone,
     emptied at each run and left to be read; `.botica/judge` ignores itself, as `.botica/cache`
     does. abide reads its key there from the environment or `~/.abide/.env`, and adds the rules
     of `~/.abide/global.json`, whose verdicts botica leaves out. A rule with `turn` or
     `pull-request` is skipped. The exit code is 0 when every fixture matches, 1 when one does not,
     and 2 when the run fails.
   - In CI, `abide check --json` reports one section per phase. A section of the `edit` phase names
     one file; the section of the `turn` phase names every file of the change. `--judged` reads the
     report: a verdict in the band `act` is a finding at the severity of the rule, one in `flag` a
     finding of severity info, and one in `clear` no finding. A verdict in `act` with a probability
     under the `act-at` of the judgment implementation, when it has one, is a finding of severity
     info too: `minimal-comments` has `act-at = 0.85`, since abide gives some files a probability
     near 0.8 on one run and under it on the next. A verdict names no line, so its
     finding is about the file of its section, the `file` of the verdict, or the whole change for
     a verdict of the `turn` phase with no `file`. `--output-format sarif` writes them as SARIF,
     and `--fail-on warning` fails the run on them, since a judgment rule reports warnings.
   - abide has two thresholds for all its rules (`act` and `flag`), so the severity of a judgment
     rule does not reach it. The diff goes to the endpoint of the key of the user (TypeSafe or the
     Vercel AI Gateway); botica sends nothing.
8. **A local model reaches abide through `TYPESAFE_AI_BASE_URL`.** With `TYPESAFE_AI_API_KEY` set,
   abide sends its calls to `<base URL>/systemone` instead of `https://api.typesafe.ai/v1`. The
   endpoint receives the model, the state (the diff, the file, the task) and one question per rule,
   and answers each question as a `choice` or a `score` with its probabilities, or as a boolean
   probability (`noul`). The server of llama.cpp serves the same API; the
   server a project uses is its own choice, and botica only passes the environment.

## Not decided

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
- The implementation of kind `judgment` in `rule.toml` has `when`, `question`, `covered-by` and
  `why-not-deterministic`, and a deterministic rule gains options with no default.
- `botica lint` reads SARIF from a judge (`--judged`), and `invalid-suppression` checks the reason,
  the level and the channel of a suppression of a judgment rule. With `--judged`, botica lints no
  file: it reports the results of the active judgment rules at the paths and in the languages they
  apply to, with their configured severity, minus the files that an `ignore-file` comment
  silences. A result with no location is about the whole change: it has no path, the text output
  shows it as `(change)`, and only the selection of the rule silences it. A result of a rule that
  is not a judgment rule of the catalog is left out with a warning on stderr.
- `botica rules export` gains `--format abide` and `--check`, and `botica rules test` gains
  `--judge abide`.
- `botica rules new <family>/<slug> --kind <kind>` writes a folder that the catalog loads: a
  `rule.toml` in preview and off by default with the next code of the family, a README with the
  required sections of its template, empty fixtures and, for `ast-grep`, the YAML of its language.
  It takes `ast-grep`, `extension` and `judgment`; a `native` rule has its code in botica-core,
  and the `external` kind comes with its fields of `rule.toml`. The `expected.json` of a judgment
  rule gives `valid/` the band `clear` and `invalid/` the band `act`. Nothing runs an
  `extension` rule of the catalog yet: `botica lint --ext <command>` runs an extension by its
  command.
- `botica rules export --format json` writes `catalog.json`, which the other exports read: its
  `schema_version`, the families, and each rule in the order of its code with every field of its
  `rule.toml`, `added`, `since`, the path of its README and the sections of the README.
- The index of rules on the site needs a search with filters by kind, family, status and language,
  as Ruff's has.
- The README of each rule follows the template of its kind, and the section that lists what a rule
  leaves alone is called Exceptions.
