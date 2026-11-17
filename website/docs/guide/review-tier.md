---
title: Review risk
---

# Review risk

`botica review tier` assesses branch risk and integration risk from pinned Git commits.
The command prints evidence. It changes no labels and performs no merge into the user's branch.

```bash
botica review tier --diff refs/remotes/origin/main --head HEAD \
  --integration-method merge --format json
```

`--diff` names the destination. `--head` defaults to `HEAD`. `--integration-method` is required:
`merge` is supported; `squash` and `rebase` return incomplete integration evidence.
`--format` accepts `json` or `text`, and defaults to `text`.
Use full ref names or full commit IDs. Short branch names are accepted only when unambiguous;
tag-only short names and branch/tag shadowing are rejected, including suffixed names such as
`main~0`. Explicit `refs/tags/<name>` references can be pinned.

Branch risk compares every best common ancestor with the head and retains every comparison.
Integration risk compares the destination tip with a synthetic merge result tree. A complete
effective tier is their maximum. An empty comparison uses medium.

The command reads review settings and attributes from the destination tip, never the head or
working copy. It uses ordinary configuration discovery and rejects dual configuration sources.
Historical settings outside `review` need not match the current configuration schema.
The output identifies the commits, integration method, botica version, selected configuration
source and BLAKE3 digest.

## Evidence and incomplete results

Each comparison lists paths, area matches, signals and their detector sources. A rename retains
both paths. Unmatched files default to core/medium; overlapping entries take the highest tier.
Forced platform paths, including root and `docs/` CODEOWNERS, cannot be hidden by a periphery
entry or generated-file glob. Forced matching is ASCII-insensitive.

Changed-line counts and files without text diffs are informational only. Generated files are
excluded from the line count, while their paths, areas and detector evidence remain present.
The inherited 400-line setting creates no gate. Only `critical` requires human review;
medium, platform and file counts create no such requirement.

The initial successful detector boundary covers documentation, review policy files and npm
manifest additions or changes outside package tables. Positive FastAPI decorators, CI and
migration paths, deleted tests, Python assertions and lowered coverage settings carry traceable
facts. General dependency and endpoint detection is still unsupported: unknown manifests,
unknown source languages and Python files with only partial FastAPI detection produce
`unsupported-detector`, preserving their comparisons without inventing a signal.
Documentation uses `.md`, `.rst` and `.adoc`, plus the basenames `README.txt`, `LICENSE.txt`,
`NOTICE.txt` and `CHANGELOG.txt`, without ASCII case. Other `.txt` files remain unsupported;
their filename alone cannot distinguish documentation from a dependency manifest.

A missing detector, invalid configuration, missing history, non-UTF-8 paths, merge conflict or
unsupported merge attribute leaves classification incomplete. `effective_tier` is null, reasons
are explicit and the exit code is 2. A technical failure blocks automation without requiring
human review. A complete classification exits 0, including high risk.

Git runs in an isolated bare repository with no user hooks, filters, textconv, external diffs or
custom merge drivers. Synthetic objects live under `.tmp/` for the duration of the command.
Custom, union and binary merge attributes are unsupported. Text-detector input is limited to
16 MiB per blob; exceeding that technical limit produces incomplete evidence.

The JSON contract is
[`schema/review-tier.schema.json`](https://github.com/collectiveai-team/botica/blob/main/schema/review-tier.schema.json).
Detector details and policy history are in
[`docs/review-risk-contract.md`](https://github.com/collectiveai-team/botica/blob/main/docs/review-risk-contract.md).
[Review planning and static checks](review-plan.md) consume this complete/incomplete contract.
Automatic merge is not implemented by these commands.
