---
name: review-contracts
description: Review role for the changes to what another program consumes that no compatibility job measures — a change of meaning with the same shape, contracts with no schema (migrations, environment variables, CLI flags, file formats, module interfaces) and new contracts with no check. Use when reviewing a branch or PR that touches something others consume, or when review-panel runs the contracts role.
---

# Review: contracts

The shape of a schema is compared by the compatibility jobs of the repository (oasdiff, griffe,
asyncapi). This role covers what those jobs do not see.

## 1. Inputs

1. Pin the target and capture `git diff <target>...HEAD`.
2. Read the role file: `git show <target>:.github/review/contracts/REVIEW.md`, or
   [`references/REVIEW.md`](references/REVIEW.md) when the repository has none.
3. List the compatibility jobs in `.github/workflows/` and `.gitlab-ci.yml` (`*-compat*`) and what
   each compares. Do not report what they compare.

## 2. Procedure

1. List each contract the diff touches: a field, a default, an error code, an event, a column, an
   environment variable, a flag, an output format, a public function. For each, name a consumer:
   another repository, a user's script, a stored file, a client.
2. For a contract with the same shape: did its meaning change? Units, defaults, order, what `null`
   or empty means, which error comes back, whether a call can repeat.
3. For a contract with no schema: is the change backward compatible for a consumer that did not
   update? For a migration: can the old code run against the new schema, and the new code against
   the old one, during a deploy?
4. For a new contract that no job compares: report it once as P3 and suggest the check that would
   cover it.

## 3. Output

Write the result as described in [`references/output.md`](references/output.md), with role
`contracts`, to `.tmp/review/<short-head-sha>/contracts.json`. Each finding names the consumer
that breaks.
