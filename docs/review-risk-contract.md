# Review risk contract

Status: the tier producer is implemented with the detector boundaries below. Public CLI tests
exercise Git repositories, and schema tests validate produced JSON. The plan/check consumers
are not implemented yet.

ADR 0018 decision 2 defines branch risk, integration risk and effective risk.
`schema/review-tier.schema.json` defines the version 1 JSON envelope. Schema validity does not
authenticate a report.

## Producer

Resolve the destination and head to commit IDs before collecting evidence. Read review
configuration from the destination tip, with the existing botica.toml/pyproject.toml precedence.
Parse and validate the review settings, rather than requiring unrelated historical settings to
fit the current whole-project schema.

The report's inputs identify the base and head commits, requested integration method, botica
version and configuration source and digest. The digest is BLAKE3 of the selected file's bytes;
`source: "defaults"` uses the empty byte string. Defaults and detector changes are also bound to
the recorded botica version.

Configuration discovery starts at the caller's directory and walks toward the repository root
in the pinned destination tree. A directory with both `botica.toml` and `[tool.botica]` in
`pyproject.toml` is invalid, as in ordinary discovery. Configuration files must be regular Git
blobs. A linked, missing-object or non-UTF-8 configuration is invalid, without a default fallback.
When the selected file's bytes cannot be read, the incomplete report records its attempted
source and a null digest. The schema permits a null digest only with both assessments incomplete
for `invalid-config`.

For branch risk, obtain every best common ancestor. Each comparison records its two Git object
IDs, tier, affected paths, areas and signals. A rename includes both paths. Preserve every
comparison as evidence; an assessment's tier is their maximum. An empty change uses a core floor
of medium, so the absence of files does not create an exception to the default risk policy.

For integration risk, compare the destination tip against the synthetic result tree of the
supported integration method. The comparison's `to` may therefore identify a tree rather than a
commit. Creating evidence must not change user refs, the index or tracked working-tree files, run hooks, or
execute code supplied by the reviewed branch. An unsupported merge driver or attribute behaviour
must leave integration incomplete rather than silently using another strategy.

The initial supported method is merge. A caller must supply the intended method; the local
command cannot infer a code host's current merge settings from Git history. Squash and rebase
return an incomplete integration assessment until supported.

Complete assessments have a tier and at least one comparison. Incomplete assessments have
nonempty reasons and any comparisons collected so far, but no aggregate tier. Both assessments
must complete for `status: "complete"` and a non-null `effective_tier`. Otherwise the report has
`status: "incomplete"`, `effective_tier: null`, and the command exits with 2.

If input resolution fails before the commits can be pinned, exit 2 with a diagnostic; do not
invent object IDs to fit the envelope. A complete classification exits 0 even when its tier is
high: high risk is a classification, not a command failure.

## Runtime evidence and detector boundary

Each comparison records per-file Git status, both rename paths, file kind, added/deleted line
counts, generated status and every matching area entry. Entry locations identify
`review.areas[index]`, `default-core` or `forced-platform`. Forced paths use ASCII-insensitive
matching, including root and `docs/` CODEOWNERS; explicit area and generated globs keep their case.
Protected paths retain full evidence and cannot be generated.

`size.changed_lines` sums additions and deletions for that comparison's non-generated text
files. `non_text_files` records binaries, symbolic links and submodules without a text size.
Size has `informational_only: true`. The user's 2026-11-16 correction supersedes ADR 0018's
original size and non-critical human-routing gates: only `critical` requires human review.
Neither 400 changed lines nor any file count creates a human-review requirement. A technical
failure blocks a complete classification without creating such a requirement.

`signal_evidence` records positive facts with paths and a detector identifier. Initial support:

- `docs-only`: every changed file is text with `.md`, `.rst` or `.adoc`, or has a basename of
  `README.txt`, `LICENSE.txt`, `NOTICE.txt` or `CHANGELOG.txt`, compared without ASCII case.
  Paths containing requirements, constraints, packages, manifest or lockfile are excluded.
  Other `.txt` names remain unsupported, since an arbitrary text filename can be a dependency
  manifest.
- `new-dependency`: npm `package.json` adds a package under dependencies, devDependencies,
  peerDependencies, optionalDependencies or overrides, or changes settings outside those tables.
  Version/constraint changes without another positive fact remain unsupported because release
  cooldown and source evidence are not implemented.
- `new-endpoint`: newly added top-level FastAPI/APIRouter decorators, with a directly imported
  constructor, a directly assigned router and a literal route path, parsed as Python syntax.
- `ci-changed`: `.github/workflows/`, `.github/actions/` or `.gitlab-ci.yml` changes.
- `migration-changed`: a path inside a `migrations` directory changes.
- `tests-deleted`: a deleted file is in `tests/` or has a `test_` basename.
- `assertions-changed`: the set of Python `assert` statements changes.
- `coverage-lowered`: numeric `tool.coverage.report.fail_under` decreases in `pyproject.toml`.

Only `new-dependency` and `new-endpoint` raise the tier to medium. Other facts select evidence
for future consumers without adding a tier rule.

`detector_evidence` distinguishes supported input from unsupported dependency/endpoint detection.
Documentation, review policy files and the supported npm cases can complete. Other inputs,
including unknown manifest names, unknown source languages and Python files with only positive
FastAPI detection, retain facts but produce `unsupported-detector`, no aggregate tier and exit 2.
This boundary intentionally limits successful classification; an absent detector cannot silently
leave an unknown dependency change at effective low risk. Generated files retain detector evidence.

Git metadata is NUL-delimited; paths are never decoded from quoted patch headers. Non-UTF-8 paths
are incomplete. An isolated bare repository reads source objects through an alternate object
directory and writes synthetic objects only under `.tmp/`. It has no user configuration, hooks,
filters, external diff or textconv commands. Destination attributes govern diff and merge.
Custom, union and binary merge attributes are unsupported. The text-detector input limit is
16 MiB per blob; exceeding it is a technical `unreadable-diff`, not a size gate.

## Consumers

`review plan` reads the effective risk only from a complete classification. It collects paths,
areas and signals from both assessments and every ancestor comparison, rather than only the
comparison that supplied the highest tier. A lower-tier comparison can still select a role.

Bind a plan to the report's base/head commits, integration method, configuration digest and
botica version. Also bind it to the trusted role definitions that the plan reads. A changed input
requires a new plan, not an updated label alone.

`review check` compares those inputs with the current trusted state before accepting evidence.
It checks the planned roles, valid outputs, remaining findings and current-head approvals.
Schema validity does not prove input freshness, that all ancestors were examined, or that
configuration came from the trusted base. Those are consumer and producer checks.

The labeling workflow must replace or invalidate a previous classification when inputs change.
An incomplete report cannot leave an earlier `risk:low` label authoritative. Neither a complete
low tier nor green labels authorize merge without ADR 0018's other gates.

## Validation seams

The public JSON schema rejects partial reports that advertise an effective tier, understated
maxima and complete assessments with no comparisons. Tests call its standard JSON Schema
validation interface in `tests/test_review_tier_schema.py`.

Producer tests use the public CLI with temporary Git repositories: ordinary changes,
multiple best common ancestors, critical reverts, conflicting integration, special path names,
trusted configuration and unsupported methods. Future plan/check acceptance tests will exercise
their public CLI inputs and observable results. The adversarial review of the accumulated slice
has not run.

The criss-cross regressions retain the supported Markdown topology case and a Python
authentication revert under a trusted `critical` area. The Python case exits with 2 and a null
effective tier because its detector coverage is incomplete. Both ancestor comparisons remain,
including the high critical comparison; integration evidence identifies the high critical revert
and the actual merge tree. The schema tests validate that incomplete source-code report.
