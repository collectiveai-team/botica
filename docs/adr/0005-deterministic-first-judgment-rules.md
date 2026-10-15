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
6. **Botica applies the suppressions of a judgment rule.** A judge reports its findings to botica
   as SARIF (`botica lint --judged <file>`), and botica filters them with the same engine,
   configuration and `overrides` as the findings of its own rules. It reports what remains. The
   export also gives the judge the suppressions, and the filter is what holds them.

   | Level | Written as | Reaches the judge as |
   |---|---|---|
   | Repository | `ignore = ["<slug>"]` | the rule is left out of the export |
   | Path | `ignores` or `extend-ignores` of the rule | globs subtracted from the `scope` of the rule |
   | Code | `# botica: ignore[<slug>] -- <reason>` on the line, `botica: ignore-file[<slug>] -- <reason>` in the file | the filter of botica |

   For a judgment rule the code channel is `botica: ignore` alone, and the reason after `--` is
   required. `noqa` does not suppress a judgment rule. `invalid-suppression` reports a suppression of
   a judgment rule with no reason, or one written as `noqa`. `unused-suppression` does not evaluate
   the suppressions of a judgment rule, since `botica lint` never runs it. A finding of a turn or of
   a pull request has no line, and only the repository and path levels silence it. The Do not flag
   section of the README lists what the rule leaves alone in every repository; a suppression is the
   exception of one repository.

## Not decided

- The formats of `botica rules export` for each consumer, and which consumer comes first.
- The classification of each standard marked `[judgment]`, one pull request per standard.

## Considered options

- **A separate tree for judgment rules** (`judgments/<family>/<slug>/`, with a code prefix of its
  own). The kind is visible from the path. Rejected: a rule that becomes deterministic changes its
  folder and its code, and the old code becomes an alias.
- **The judge applies the suppressions,** told by the export to skip a line that carries the
  comment. Rejected: a model may ignore the instruction, and the suppression would stop being
  deterministic.
- **A judgment rule for every standard that depends on the repository.** Rejected: a model would
  give a probable answer where the configuration of the repository gives an exact one.

## Consequences

- The kind table of ADR 0003 describes `judgment` as in this ADR.
- `rule.toml` gains `when`, `question`, `covered-by` and `why-not-deterministic` for judgment rules,
  and options with no default for deterministic rules.
- `botica lint` reads SARIF from a judge (`--judged`), and `invalid-suppression` checks the reason
  and the channel of a suppression of a judgment rule.
- The index of rules on the site needs a search with filters by kind, family, status and language,
  as Ruff's has.
- The README of each rule follows the template of its kind, and the section that lists what a rule
  leaves alone is called Exceptions.
