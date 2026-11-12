---
name: review-panel
description: Runs the review-<role> skills on a diff — picks the roles, runs each in its own subagent, runs review-refute last, and merges what survives into one report. Use when the user asks for a review by roles or a full review of a branch or PR, or before opening a PR whose change is larger than a typo.
---

# Review panel

This skill orders the review roles. Each role is its own skill and its own subagent, so that a role
does not see the reasoning of another before `review-refute` reads them all.

## 1. Pick the roles

1. Pin the target and the head: `git rev-parse --short HEAD` is the folder name of the run,
   `.tmp/review/<short-head-sha>/`. Delete that folder first if it is there from an earlier run.
2. When `botica review plan` exists (`botica review --help`), run
   `botica review plan --diff <target> --json` and take its roles. Otherwise pick them by hand:

   | Role | Runs when |
   |---|---|
   | `spec` | always |
   | `security` | always |
   | `tests` | the diff changes source files, test files or what decides which tests run (CI, hooks, the settings of the test runner) |
   | `architecture` | the diff adds a module, a type that others use, or an import between packages |
   | `contracts` | the diff changes something another program consumes |
   | `performance` | the diff changes a query, a job, a hot path or a loop over large data |

3. Say which roles run and which do not, with the reason, before starting.

## 2. Run the roles

Start one subagent per role, in parallel. Give each one only: the target, the head, the path of the
skill `review-<role>` to follow, and the folder of the run. Do not give it the findings of another
role or your own opinion of the diff.

Wait for all of them. A role that ended without writing its JSON counts as `INCONCLUSIVE`.

## 3. Refute

Start one more subagent, in a fresh context, with the skill `review-refute`, the target, the head
and the folder of the run. It reads the JSON of the roles and writes `refute.json`.

## 4. Report

Print one table with the findings that `refute.json` marked `confirmed`, the most severe first:
severity, role, file and line, title, and how to verify. Under it, list:

- the verdict of each role;
- the findings that were refuted, one line each with the reason;
- what each role put in `not_checked`.

The run fails when a confirmed finding reaches the blocking severity of its role. Then the change
is fixed, or the user accepts the finding, before a PR goes out. Leave the folder of the run in
`.tmp/`: it is the input that `botica lint --reviewed` will read. The format of each file is
[`references/output.md`](references/output.md).

Credits and sources: [`references/CREDITS.md`](references/CREDITS.md).
