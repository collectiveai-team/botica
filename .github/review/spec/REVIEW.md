---
role: spec
applies:
  paths: []
  tiers: [low, medium, high]
stages: [local]
blocking: P1
owners: ["@jedzill4"]
max-findings: 10
---

## Invariants

- A change does what its issue asks, and nothing the issue does not explain.
- A change that the issue names an ADR or a standard for follows its decision as written on the
  base commit, under `docs/adr/` or `standards/`.
- A change to a command of botica updates its page under `website/docs/` (ADR 0012).
- A skill names a rule by its slug and states no rule of its own (ADR 0008).

## Check always

- Each acceptance criterion of the issue has code and evidence in the diff: a test that the role
  ran, or a file of the diff that shows it. A result written in the description of the pull request
  is not evidence.
- The description of the pull request or its commits link the issue.
- A criterion that an ADR or a standard sets has the same evidence as one that the issue states.
- The diff changes nothing that the issue does not explain, except a refactor the change needs and
  the description says so.

## Do not report

- Style, naming and formatting.
- A finding of another role (security, tests, architecture, contracts, performance).
- A criterion that the issue marks as out of scope or for a later issue.
- The prose of a page, a standard or an ADR: `plain-writing`, `minimal-comments` and
  `ubiquitous-language` in `.abide/rubric.json` cover it, and `abide.yml` judges them.

<!-- botica:covered -->
- The job `rule contract` of `ci.yml` (`scripts/check_rule_change.py`): a change that adds a rule
  and touches more than the rule needs, or touches more than one standard.
- The job `validate PR title` of `conventional-commits.yml`: the title of the pull request.
<!-- /botica:covered -->

## Evidence

Each finding quotes the criterion and the issue, ADR or standard it comes from, and points at the
file and line, or says that no line exists.
