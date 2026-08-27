# CES-110 · cognitive complexity (complexipy)

**Code:** `CES-110` &nbsp;·&nbsp; **Slug:** `cognitive-complexity-complexipy` &nbsp;·&nbsp;
**Enforced by:** prek hook &nbsp;·&nbsp; **Tracker:**
[#110](https://github.com/collectiveai-team/scaffolding/issues/110)

## Directive

A function's [SonarSource-style Cognitive Complexity](https://www.sonarsource.com/resources/cognitive-complexity/)
score must stay under 15, enforced by
[`complexipy --max-complexity-allowed 15`](https://github.com/rohaquinlop/complexipy).

## Why

Ruff's `C90`/mccabe (`max-complexity = 10` in this house's `pyproject-template.toml`) counts
branches — **cyclomatic** complexity only. Cognitive complexity instead penalizes nesting depth
and control-flow breaks — the metric that tracks how hard a human actually finds a function to
read. An agent that keeps patching one function with more special-cased `if`/`try` nesting instead
of extracting helpers can stay ruff-clean while becoming genuinely harder to follow. `complexipy`
and ruff's `C90` are complementary, not redundant — keep both.

## Threshold

15, at **error tier** in the shipped hook. New repos get this from a clean baseline; existing
repos should baseline first (see Migration below) rather than big-bang fixing everything at once.

## When you hit a violation

1. Find the nesting/branching that's driving the score (deep `if`/`elif`/`try` chains, nested
   loops, boolean operator chains).
2. Extract the inner logic into a named helper function — this usually also improves the
   function's testability (pairs with CES-65 test-through-interface).
3. Re-run the hook to confirm the function is back under budget.

## Migration (existing repos)

Use `complexipy --snapshot-create` to baseline current violations before enabling the hook as a
hard error, so pre-existing complexity doesn't block unrelated PRs — this mirrors how CES-5's
import-linter skeleton is meant to be filled in gradually, and CES-71's warn-then-error two-tier
shape for `file-size-guard`.

## Suppression

`complexipy` is a prek hook with no per-function ast-grep ignore. If a single function
legitimately needs to stay complex (a generated parser, for example), exclude its file via the
hook's file-pattern configuration rather than raising the global threshold.
