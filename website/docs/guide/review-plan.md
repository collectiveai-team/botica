---
title: Review planning and static checks
---

# Review planning and static checks

`botica review plan` selects roles from destination-owned `REVIEW.md` frontmatter.
`botica review check` validates that policy and local skill availability. Neither runs a role,
accepts findings, counts approvals, changes labels or authorizes merge.

```bash
botica review plan --diff refs/remotes/origin/main --head HEAD \
  --integration-method merge --stage local --format json
botica review check --diff refs/remotes/origin/main --head HEAD \
  --integration-method merge --stage ci --format json
```

Both commands use the pinned inputs and Git boundary of [Review risk](review-tier.md).
`--head` defaults to `HEAD`, `--stage` to `local` and `--format` to `text`.
The integration method is explicit. The commands recompute classification; they do not import
an unauthenticated risk artifact.

## Trusted roles and selection

Roles come from `[review] dir`, or `.github/review/` or `.gitlab/review/` on the destination.
If both host directories exist, set `dir` explicitly. Configured roles default to the seven
roles of ADR 0017. Both commands require their destination definitions, without falling back
to the head or an installed template.

`botica recipe install base` installs generic role definitions and the review skills.
Commit the definitions on the destination before planning a later change. The templates declare
empty automated coverage; configure the repository's actual checks before claiming coverage.
Installation preserves customized policy files and reports differing templates as pending.

```yaml
---
role: contracts
applies:
  paths: ["api/**"]
  tiers: [medium, high]
  signals: [new-endpoint]
stages: [local, ci]
blocking: P1
owners: ["@org/platform"]
max-findings: 10
covered: {}
---
```

The role must match its folder. Stages are `local` and `ci`; blocking is `P0` through `P3`;
`max-findings` is a positive integer. Unknown keys fail validation.

Selection requires every configured criterion: a matching stage, path, effective tier and signal.
Paths and signals come from every comparison in both assessments, including lower-tier
ancestors. Empty/missing paths mean the whole diff; empty/missing signals impose no signal
filter. Missing tiers allow every tier, while an explicit empty tier list selects none.
Defaults are both stages, blocking `P1`, no owners and 10 findings.

A complete classification is required for a final plan. If classification or trusted policy
is incomplete, `selected_roles` is null, evidence remains visible and the exit code is 2.
Unknown detector facts remain unknown. A valid complete plan exits with 0.

The report records base/head IDs, integration method, botica version and configuration digest.
It also binds the stage, review directory, classification and trusted role-folder files, shared
README and skills manifest. Absent optional README/manifest files have null digests. Changes
require recomputation; schema validity alone does not establish authenticity or freshness.

## Static check

`review check` checks every configured role, even one that the current stage would skip:

- frontmatter syntax, names, enums, owner syntax and path globs;
- each path glob matches at least one file in the pinned destination or head tree;
- declared coverage sources exist and the generated `botica:covered` block is current;
- `.agents/skills/review-<role>/SKILL.md` is a nonempty regular file, with no symbolic link in its
  path. Its content is fingerprinted, never executed or treated as instructions by the command.

A complete static check exits with 0 even when the embedded classification is incomplete.
It never publishes a final `selected_roles` list. This status means policy validation passed,
not that the change was reviewed or can merge. Static errors exit with 2.

## Coverage declarations

Coverage is declared explicitly, without inferring a role-to-check mapping:

```yaml
covered:
  rules: [file-size-guard]
  hooks:
    - file: prek.toml
      id: botica
  jobs:
    - file: .github/workflows/ci.yml
      id: test
```

Rules use canonical active slugs from trusted configuration/catalogs. Hooks use exact IDs under
`repos[*].hooks`; GitHub jobs use exact keys under `jobs`. A top-level GitLab job must contain
`script`, `trigger` or `extends`. Remote includes and dynamic definitions remain unverified.
Source paths are literal and repository-relative; symbolic links and traversal are refused.

`covered: {}` explicitly declares no exclusions. Missing declarations leave legacy coverage
unverified. The commands return `covered.expected_block`; copy that block into a trusted
destination revision to make it current. Existing explanatory prose belongs outside the
generated markers. Stale or unverified coverage blocks the operation.

Fingerprints establish configured presence/freshness, not that a check ran, passed or covers
all role invariants. Hook/job fingerprints bind whole source files; rule fingerprints also bind
configuration, catalog definitions, project catalog sources and the botica version.
The bounded format is documented in
[`docs/review-coverage-contract.md`](https://github.com/collectiveai-team/botica/blob/main/docs/review-coverage-contract.md).

Only changes touching `critical` require human review. These commands add no human-routing or
size rule. Technical/configuration errors block the operation without requiring a person.
