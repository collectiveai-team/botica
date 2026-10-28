# A rule keeps its code and its id across releases

**Status:** proposed

This is step 2 of the discussion "How a project moves to a new release of botica"
(<https://github.com/collectiveai-team/botica/discussions/261>).

A project names rules by id or by code in `botica.toml`, on the command line and in suppression
comments, and moves from one release of botica to another with that text unchanged. ADR 0002 says
that a renamed id keeps an alias, and says nothing about codes. Releases 0.9.0 and 0.13.0 changed
the code of thirteen rules, and 0.13.0 and 0.16.0 gave nine of the old codes to other rules. A
configuration that names one of those codes selects, ignores or suppresses another rule after the
update, and no message says so. The notes of v0.13.0 hold only the link to the full changelog.

Each `catalog/rules/*/*/rule.toml` of each tag from v0.7.0 (the first with codes; up to v0.12.0
under `lint/rules/`) compared with the same file at the tag before gives:

| Release | Rule | Code before | Code after | What the old code names after |
| --- | --- | --- | --- | --- |
| 0.9.0 | `api-schemas-extra-forbid` | `BTDM001` | `BTFAST001` | nothing; `no-dict-alias` from 0.13.0 |
| 0.10.0 | `no-console-log`, merged into `log-no-print` (c1f8929) | `BTLOG003` | none | nothing |
| 0.13.0 | `arch-database-package`, renamed `database-package` with an alias | `BTARCH001` | `BTDB001` | `import-layers` |
| 0.13.0 | `settings-module` | `BTARCH002` | `BTCONF001` | `separate-orchestration` |
| 0.13.0 | `settings-base-class` | `BTARCH003` | `BTCONF002` | `architecture-vocabulary` |
| 0.13.0 | `settings-direct-instantiation` | `BTARCH004` | `BTCONF003` | nothing; `no-eager-heavy-imports` from 0.16.0 |
| 0.13.0 | `db-no-create-all` | `BTARCH005` | `BTDB002` | nothing |
| 0.13.0 | `settings-env-schema` | `BTARCH006` | `BTCONF004` | nothing |
| 0.13.0 | `import-layers` | `BTARCH007` | `BTARCH001` | nothing |
| 0.13.0 | `no-dict-alias` | `BTDM002` | `BTDM001` | `no-dict-call-return` |
| 0.13.0 | `no-dict-call-return` | `BTDM003` | `BTDM002` | `no-dict-literal-return` |
| 0.13.0 | `no-dict-literal-return` | `BTDM004` | `BTDM003` | `no-dict-return-annotation` |
| 0.13.0 | `no-dict-return-annotation` | `BTDM005` | `BTDM004` | nothing |
| 0.13.0 | `cli-typed-framework` | `BTPROC001` | `BTCLI001` | `pr-small-by-intent` |

No other code or id changed or disappeared between v0.7.0 and v0.17.1. Two ids left the catalog:
`arch-database-package`, which resolves through its alias, and `no-console-log`, which is an
unknown rule today. Most of 0.13.0 comes from 27fef3b, which moved rules to the families `cli`,
`config`, `database` and `data-models` and numbered each family again from 001.

What botica does today with these names:

- `Catalog::resolve` (`crates/botica-catalog/src/load.rs`) looks up an id, a code or an alias in one
  index. Every place that reads the name of a rule goes through it: `select`, `extend-select` and
  `ignore` (`crates/botica-config/src/selector.rs`), `[rules.<id>]` and `[[overrides]]`
  (`validate.rs`, `resolve.rs`), and the suppression comments, whose spellings are the code and the
  aliases of each rule (`crates/botica-cli/src/project/policy.rs`). An alias resolves with no
  message.
- A name that the index does not hold stops the load of the configuration with exit 2:
  ``select[0]: unknown rule `BTARCH005` ``, or ``ignore[0]: unknown rule `no-console-log` ``.
  `botica lint`, `botica rules` and `botica recipe check` all stop there. A rule with
  `status = "removed"` stops it with ``rule `<id>` was removed and cannot be mentioned``. In a
  comment, an unknown name is
  a finding of `invalid-suppression`.
- `catalog/rules/history.toml`, written by `scripts/stamp_release.py`, records only `version`,
  `rule` and `status`, and `check_history` (`crates/botica-catalog/src/validate.rs`) rejects an
  event whose rule is not in the catalog. No file records the code a rule had in a release.
- `validate_across.rs` rejects a code that two rules of the catalog share and an alias that
  collides with a live id or code. It cannot see a code that only an older release used.
- `next_code` in `crates/botica-cli/src/rules_new.rs` says "a code is never reused" and takes the
  highest code of the family in the current catalog plus one. With botica 0.17.1, `botica rules new
  architecture/x`, `data-models/x` and `logging/x` propose `BTARCH005`, `BTDM005` and `BTLOG003`:
  three codes that a release published for other rules.

## Decision

1. A code or an id that a release published names the same rule in every later release. Published
   means present in a `rule.toml` of a release tag. The rule keeps it as its code or id, as a
   retired name that resolves to it, or, once removed, in the `rule.toml` that stays with
   `status = "removed"`. No other rule takes it as its code, id or alias. A code or id that changed
   before any release named it is free: `version-pin` came in c637527 and left in 52ea4a5, both
   between v0.12.0 and v0.13.0.
2. A renamed rule, or one whose code changes, keeps its old id and its old code as retired names.
   botica resolves a retired name to the rule wherever it reads the name of a rule: `select`,
   `extend-select`, `ignore`, `[rules.<name>]`, `[[overrides]]`, `replace`, `--select`,
   `--extend-select`, `--ignore`, `# botica: ignore[...]`, `# botica: ignore-file[...]`,
   `# noqa: <code>`, `botica explain` and `botica refs show`. Each run prints one warning on stderr
   for each retired name it read, with the current id and code and the release that changed them:
   ``botica: warning: `BTARCH005` is the old code of `db-no-create-all` (BTDB002 since 0.13.0)``.
   Retired names resolve for good. A project can skip any number of releases, a retired name is
   one more key in the index of `Catalog::resolve`, and ruff still redirects codes that its source marked "Remove
   by 2023-02-01" (`crates/ruff_linter/src/rule_redirects.rs`). The warning gets the configuration
   rewritten over time without breaking a run.
3. A removed rule keeps its folder, with `status = "removed"` in its `rule.toml` and its README.
   A name of a removed rule that turns it on (`select`, `extend-select`, `--select`,
   `--extend-select`, `enabled = true` in `[rules.<name>]`) stops `botica lint` with exit 2, as
   today, and the message names the release that removed it and the rule whose `replaces` names
   it, if any. The project asked for a check that no longer runs, and a run that went on would drop
   the check with no failure. A name of a removed rule anywhere else (`ignore`, `--ignore`, the
   other keys of `[rules.<name>]`, `[[overrides]]`, a suppression comment) is a warning and
   changes nothing, since a rule that does
   not run reports nothing to silence. ruff does both: it fails on a removed rule in `select` and
   warns on one in `ignore` (`crates/ruff_workspace/src/configuration.rs`). `botica rules`,
   `botica explain` and `botica recipe check` print the error of a removed rule as a warning and go
   on, so the commands that describe the change run. A name that no release published, such as a
   typo, stays an error with exit 2 in every command.
4. `catalog/rules/history.toml` records codes and renames. An event names the rule by the id it
   has at `version`, and carries a `status`, a `code`, a `renamed-from` or more than one of them:

   ```toml
   [[event]]
   version = "0.13.0"
   rule = "db-no-create-all"
   code = "BTDB002"

   [[event]]
   version = "0.13.0"
   rule = "database-package"
   renamed-from = "arch-database-package"
   code = "BTDB001"
   ```

   A `code` event says that from `version` the rule has that code, and the code of its previous
   `code` event is retired. A `renamed-from` event says that from `version` the rule has the id
   `rule`, and the events before it name the rule by the old id. The retired names of a rule are
   the codes and ids of its events that it no longer has and that no rule of the catalog has. The
   binary embeds the history, as it does today, and adds the retired names to the index of
   `Catalog::resolve`.

   The author of a change edits only the `rule.toml`: a new code in `code`, or a new id in `id` with
   the old id added to `aliases`, as ADR 0002 asks. `scripts/stamp_release.py stamp <version>`
   writes the events at release time: a `code` event for a rule with no code in the history or
   with a code other than the last one, and a `renamed-from` event for an alias that the history
   records as the id of a rule and that no `rule.toml` holds. `stamp_release.py check`, which
   `botica-release.yml` runs before it builds, fails while one of them is missing.
   The history of a removed rule stays, since its `rule.toml` stays.

   Three checks reject the reuse of a published code or id:
   - `check_history` fails when a rule has, in its `rule.toml` or in an event, a code, an id or an
     alias that an event gives another rule, unless the history records both by 0.17.1. It runs on
     every load of the built-in catalog, so `cargo test` fails.
   - `stamp_release.py stamp` refuses to write an event that `check_history` would reject.
   - `botica rules new` takes the highest code of the family among the current codes and the
     codes of the history, plus one.

   A test of `botica-catalog` asserts that no retired name collides with a live code, id or alias,
   as `overshadowing_redirects` does in ruff.
5. The reuses published up to 0.17.1 stay as they are. A code that a rule of the catalog holds
   names that rule: `BTPROC001` is `pr-small-by-intent`, and `BTARCH001` to `BTARCH004` and
   `BTDM001` to `BTDM004` name the rules that hold them today. It does not resolve to the rule
   that held it before. A retired code that no rule holds resolves to the last rule that held it:
   `BTARCH005` to `db-no-create-all`, `BTARCH006` to `settings-env-schema`, `BTARCH007` to
   `import-layers` and `BTDM005` to `no-dict-return-annotation`. A one-time change writes into
   `history.toml` the code and the id of each rule at each tag from v0.7.0 to v0.17.1, with the
   reuses of 0.13.0 and 0.16.0 as they happened, which is why `check_history` accepts what the
   history records by 0.17.1.
   `no-console-log` gets back its folder with `status = "removed"` and the code `BTLOG003`, and
   `log-no-print`, which took over its TypeScript check, names it in `replaces`. `no-console-log`
   is not an alias of `log-no-print`: `log-no-print` also reports `print` in Python, and an alias
   would widen what an old `select` turns on.
6. `botica rules --since <version>`, which lists the rules that the releases after `<version>`
   added, promoted, renamed or removed, reads the history that decision 4 writes. Its options and
   its output are decided in the change that adds it.

## Not decided

A catalog of a project (`.botica/rules/<name>/`) has no `history.toml`, so this ADR covers the
built-in catalog. A catalog of a project keeps aliases in its `rule.toml` as before.

## Considered options

- Keep retired names for N releases and then drop them. Rejected: a project that updates after
  more than N releases gets the silent change of meaning that this ADR removes, and the entry costs
  one key of an index. ruff planned to drop its redirects and kept them.
- Retired names with no warning. Rejected: the configuration would keep names that a reader no
  longer finds in `botica rules`, and nothing would move it to the current ones.
- Every retired code written by hand in the `aliases` of its rule. Rejected: the author of a
  change that moves a family has to remember each old code, as in 27fef3b, and a missing one goes
  unnoticed. `stamp_release.py` derives the codes from the catalog and the history.
- A file of its own for renames and codes, apart from `history.toml`. Rejected: the history is
  already the record of what each release did to a rule, it is embedded, and it is written by the
  release.
- A removed rule in `select` as a warning. Rejected: the project would lose a check and see a
  passing run.
- A removed rule in `ignore` as an error, as today. Rejected: the project already has what it
  asked for, and the error stops the commands that would explain the change.
- Restore the meaning that the codes had before 0.13.0. Rejected: a project that installed 0.13.0
  or a later release and wrote `BTPROC001` meant `pr-small-by-intent`, and every release from
  0.13.0 to 0.17.1 shipped the current meaning.

## Consequences

- A change that renames a rule or changes its code no longer breaks a project, and needs no `!`
  in the title of its pull request (`docs/botica-release.md`). A change that removes a rule still
  breaks a project that selects it.
- A family is never numbered again. A rule that moves to another family gets the next code of
  that family, and its old code stays reserved, so a family can have gaps in its codes.
- A project that wrote `BTPROC001`, `BTARCH001` to `BTARCH004` or `BTDM001` to `BTDM004` before
  0.13.0 and kept the text selects, ignores or suppresses another rule than it meant, and no
  warning says so, since those codes name live rules. The notes of v0.13.0 get a section
  `## Breaking changes` with the table of this ADR.
- `history.toml` grows by one event for each new rule and each change of code or id.
- `Catalog::resolve` returns the rule and says whether the name was retired, so each place that
  reads a name can print the warning of decision 2.

The changes that follow, one pull request each: `stamp_release.py` and `check_history` record and
check codes and renames, with the events of the past and the folder of `no-console-log`; the
loader resolves retired names with a warning and treats a removed rule as decision 3 says;
`botica rules new` reads the history; and `botica rules --since`.
