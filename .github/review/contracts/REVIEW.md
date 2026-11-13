---
role: contracts
applies:
  paths: []
  tiers: [medium, high]
stages: [local, ci]
blocking: P1
owners: ["@jedzill4"]
max-findings: 8
---

## Invariants

The contracts of botica, each with the programs and the people that consume it:

- The commands of the command line, their flags and their exit codes (`0` clean, `1` a finding at
  or above `--fail-on`, `2` the run failed; `website/docs/guide/lint.md`). Consumers: the hooks of
  the mirror (`distribution/botica-pre-commit/.pre-commit-hooks.yaml`), the hooks of `prek.toml`,
  the workflows that the recipes install (`catalog/recipes/files/`), which run `uvx botica@<version>`
  with a pinned release, and the scripts of the users.
- The output formats of `botica lint --output-format`: `json`, with its `schema_version`
  (`SCHEMA_VERSION` of `crates/botica-core/src/report/mod.rs`); `sarif` (2.1.0), which GitHub code
  scanning reads; `github`, the workflow commands of GitHub Actions; `markdown`, the summary of a
  job; and `text`. Consumers: CI, editors, code scanning and the scripts of the users.
- The configuration: `botica.toml` and `[tool.botica]` of `pyproject.toml`, whose keys
  `crates/botica-config/src/model.rs` declares and `schema/botica.schema.json` publishes. An
  unknown key fails the run (`deny_unknown_fields`), so a key removed or renamed with no alias
  breaks every repository that sets it.
- The ids, the codes and the aliases of the rules, which a configuration, a `# botica: ignore[...]`
  comment, a `# noqa: <code>` comment and the baseline name: a published code or id never names
  another rule, and a renamed rule keeps an alias (ADR 0016, `catalog/rules/history.toml`).
- The extension protocol (`PROTOCOL_VERSION` of `crates/botica-ext/src/lib.rs`), its requests,
  their fields and the environment of the process (`BOTICA_EXTENSION_TIMEOUT`), and the SDKs that
  speak it: `sdk/python` (the module `botica.sdk` of the wheel), `sdk/node`, `sdk/go` and
  `sdk/rust`. Consumers: the extension rules of the projects, which a release of botica runs with
  the SDK that the project pinned.
- `botica server`: the Language Server Protocol and its `initializationOptions`. Consumers: the
  editors (`website/docs/guide/editor.md`).
- The files that botica writes and reads again in a later run or a later release: the history
  `.botica/history/runs.jsonl` and `findings.jsonl` (`VERSION` of `crates/botica-core/src/history.rs`,
  ADR 0019), the baseline of known findings (`--write-baseline`), the result cache
  (`.botica/cache`), the block `botica:rules` of `AGENTS.md`, `.abide/rubric.json`, the files of
  `botica review export` and the verdicts that `--judged` reads.
- The recipes: their names, their parameters (`[recipes.params]`) and the paths of the files they
  install, which `botica recipe check` compares in every repository that installed them (ADR 0006).
- The environment variables that botica reads: `BOTICA_HISTORY_KEY`, `BOTICA_EXTENSION_TIMEOUT`,
  `BOTICA_REFS_TIMEOUT`.
- The paths of the skills under `skills/<group>/<name>/SKILL.md`, which the `skills-lock.json` of
  each repository that installed them records (`AGENTS.md`).
- The JSON of the review roles (`schema/review-findings.schema.json`, `.github/review/README.md`).

## Check always

- A default of a flag, of a key of the configuration or of an option of a rule that changes, and a
  rule whose `default-enabled` or `default-severity` changes: the same configuration gives a
  different result after the update.
- A change to what an exit code means, to the order of the findings in an output, or to what an
  empty list, `null` or a missing key means in an output or in the configuration.
- A field of the `json` output, of the `sarif` output, of the history or of the baseline that
  changes its meaning or its unit, or is removed, without a new `schema_version` or `VERSION`.
- A flag, a key of `botica.toml`, a parameter of a recipe or an environment variable removed or
  renamed with no alias, and a value it accepted that it now refuses.
- A code, an id or an alias of a rule that changes, or that names another rule, with no event in
  `catalog/rules/history.toml`.
- A request, a field or a reply of the extension protocol that changes without a new
  `PROTOCOL_VERSION`, or a change to one SDK that the other three do not get.
- A path of a file that a recipe installs, or of a skill under `skills/`, that changes.

## Do not report

- What the compatibility jobs and tests compare (see the block below).
- A new flag, key, field or output with a default that keeps the old result.
- A change to the text of a message meant for a person, which no program parses.

<!-- botica:covered -->
- The job `public API of the package` of `api-compat-python.yml` (griffe): the public API of the
  module `botica.sdk` of `sdk/python`.
- The test `the_published_schema_is_the_generated_one` of `crates/botica-config/tests/schema.rs`,
  run by the job `fmt, clippy & test` of `ci.yml`: `schema/botica.schema.json` matches the keys of
  the configuration. It does not compare the schema with the one of the base.
- The checks of `catalog/rules/history.toml` that `crates/botica-catalog/src/validate_history.rs`
  runs when the catalog loads, with the tests of `crates/botica-catalog/tests/history_names.rs`: a
  published code or id that names another rule (ADR 0016).
- The tests of the output formats in `crates/botica-cli/tests/output.rs`, of the protocol in
  `sdk/rust/tests/protocol.rs`, `sdk/go/botica_test.go` and `crates/botica-cli/tests/node_sdk.rs`
  and `python_sdk.rs`, and of the schema of the review roles in
  `tests/test_review_findings_schema.py`. They check the behaviour of `HEAD`, not that it matches
  the base, so a change of meaning that updates its test is still reviewed here.
<!-- /botica:covered -->

## Evidence

Each finding names the contract, the consumer and the input on which the consumer gets a different
result on the base and on the head, with file and line on both sides.
