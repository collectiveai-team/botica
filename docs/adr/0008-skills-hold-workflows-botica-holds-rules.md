# Skills hold workflows; botica holds the rules

**Status:** accepted

The skill `engineering-rules` carried 67 rule files. An audit on 2026-10-20
(`.tmp/audit/triage-v2-{A,B,C}.md`, outside git) found 8 of them copied from a standard, 7 copied
from a skill that the recipe `base` installs, 4 copied from a recipe, and others with deprecated
APIs or code that fails when it runs. A copy drifts from its source, and an agent that reads both
gets two answers.

## Decision

1. The text of a rule lives in botica only: a standard in `standards/`, a rule in
   `catalog/rules/`, code to follow in a topic of `catalog/references/`.
2. A skill describes a workflow: the steps, the order, and the commands and skills each step runs.
   It names a rule by its slug and gets its text from `botica rules`, `botica explain <rule>` and
   `botica refs show <rule or topic>`. It states no rule of its own and copies no text of a rule.
3. A skill that held rules is reduced to its workflow or removed. `engineering-rules` is removed;
   `engineering-pr-review` and `engineering-refactor` keep their steps and point at botica, `pr`
   and the upstream skills `code-review` and `improve-codebase-architecture`.

## Considered options

- **Keep the skill, generated from the catalog** (ADR 0005, decision 5). Rejected: a generated copy
  is still a second place to read, and `botica explain` gives the same text on demand.
