---
name: Engineering rule proposal
about: Propose a rule/standard to bake into the scaffolding templates
title: "area/rule-id: <imperative one-liner>"
labels: ["state:proposal"]
---

<!--
Add the remaining labels by hand (or let the seed script do it):
  area:<architecture|standards|infra|impl/...>   enforcer:<ast-grep|import-linter|ruff|pytest|kube-linter|script|llm>
  priority:<high|medium|low>
The issue NUMBER is the stable id. Priority/state are labels, never the number.

Do NOT set state:approved or state:declined by hand — they are derived from the
vote below and any manual value is overwritten on the next tally.
-->

## How this gets decided

Comment one of these, **on a line of its own** — the command must start the line.
`` `/approve` `` and `**/approve**` are fine; `> /approve` (quoting someone) and
`/approved` are not.

| Command | Effect |
|---|---|
| `/approve` | Support it. Re-running replaces your vote; you get **one vote per account**. |
| `/object <reason>` | Veto. Blocks approval regardless of how many approve. Only **you** can lift it. |
| `/withdraw` | Retract your standing vote. |
| `/decline` | Vote to reject. Needs quorum — silence never declines. |
| `/upvote` | Signal support. Anyone may use it; advisory only, it never approves anything. |

- **Approved** at **2 approvals**, or 1 approval + **7 days** with no objection
  (tagged `approved:lazy`, so unread standards stay auditable). The clock starts at
  the first approval, not at the issue date.
- Only accounts with **write access** are counted for approve/object/decline.
- Progress shows on the board: `waiting-review` once someone approves, `blocked` while an
  objection stands, `upvote:N` for advisory support (`/upvote` + thumbs-up reactions).
- **Revising the body dismisses approvals** — they were cast against text that no
  longer exists. Objections survive; a decline is final.
- `/update-proposal <instructions>` revises this proposal in place. It cannot approve
  or decline it.

## Summary
<!-- One sentence: what this rule mandates. -->

## Rule / decision
<!-- The precise, testable statement of the rule. -->

## Enforcement
- **Enforcer:** <ast-grep | import-linter | ruff | pytest | kube-linter | script | llm>
- **Tier:** <deterministic | hybrid | judgment>  <!-- derived: only llm = judgment; llm+tool = hybrid; no llm = deterministic -->
- **Applies to:** <all | fastapi | nextjs | prefect | asr | k8s | ...>

<!-- The ACTUAL change/config that implements the rule: the pyproject.toml block,
     import-linter contract, ast-grep YAML, kube-linter check, shell snippet, etc. -->
```toml
# e.g. the block added to pyproject.toml
```

## Reasoning
<!-- Why. Semantic anchor if any; cross-repo evidence; what problem it prevents. -->

## Conflicts & risks
<!-- Collisions with existing conventions (esp. respect-local-repo), false positives,
     overlap/duplication with other proposals. -->

## Blast radius
<!-- Scope of impact: which repos/layers/files; expected noise; CI vs pre-commit;
     error vs warning. How disruptive is turning this on? -->

## Migration (existing repos)
<!-- How to adopt where the rule is NOT yet satisfied:
     opt-in per repo, grandfather (ignore_imports / baseline), codemod, phased rollout,
     or "new repos only". import-linter & other whole-repo checks MUST address this. -->

## References
<!-- Provenance file, related proposals (#ids), semantic anchor links. -->

## Changelog
<!-- One-line summary per revision, oldest first. `/update-proposal` appends here. -->
- Initial proposal.
