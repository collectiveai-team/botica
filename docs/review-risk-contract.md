# Review risk contract

Status: risk semantics agreed on 2026-11-16; the JSON envelope has schema tests. The tier
producer and its plan/check consumers are not implemented yet.

ADR 0018 decision 2 defines branch risk, integration risk and effective risk.
`schema/review-tier.schema.json` defines the version 1 JSON envelope. The schema is not a claim
that the tier command already produces this output, and it does not authenticate a report.

## Producer

Resolve the destination and head to commit IDs before collecting evidence. Read review
configuration from the destination tip, with the existing botica.toml/pyproject.toml precedence.
Parse and validate the review settings, rather than requiring unrelated historical settings to
fit the current whole-project schema.

The report's inputs identify the base and head commits, requested integration method, botica
version and configuration source and digest. The digest is BLAKE3 of the selected file's bytes;
`source: "defaults"` uses the empty byte string. Defaults and detector changes are also bound to
the recorded botica version.

For branch risk, obtain every best common ancestor. Each comparison records its two Git object
IDs, tier, affected paths, areas and signals. A rename includes both paths. Preserve every
comparison as evidence; an assessment's tier is their maximum. An empty change uses a core floor
of medium, so the absence of files does not create an exception to the default risk policy.

For integration risk, compare the destination tip against the synthetic result tree of the
supported integration method. The comparison's `to` may therefore identify a tree rather than a
commit. Creating evidence must not change refs, the index or the working tree, run hooks, or
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

The envelope records comparison summaries. Per-file match explanations, detector evidence and
size gates required by ADR 0018 remain part of the producer work. Their detailed schema and the
aggregation of size across multiple ancestor comparisons must be settled before publication.
Passing the envelope schema alone does not satisfy ticket 12.

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

The next producer tests use the public CLI with temporary Git repositories: ordinary changes,
multiple best common ancestors, critical reverts, conflicting integration, special path names,
trusted configuration and unsupported methods. Plan/check acceptance tests exercise their public
CLI inputs and observable results. No adversarial slice review is claimed before those behaviours
exist and the slice can be exercised end to end.
