---
id: CES-110
title: Cognitive complexity
---

## Directive

The
[Cognitive Complexity](https://www.sonarsource.com/resources/cognitive-complexity/) score of a
function stays under 15, checked by
[`complexipy --max-complexity-allowed 15`](https://github.com/rohaquinlop/complexipy).

## Why

Ruff's `C90` (mccabe) counts branches, which is cyclomatic complexity. Cognitive complexity
penalizes nesting depth and breaks in control flow, and it tracks how hard a person finds a
function to read. An agent that keeps patching one function with more special-cased `if` and `try`
nesting, and does not extract helpers, can stay clean for ruff while the function gets harder to
follow. `complexipy` and `C90` measure different things and both stay on.

## Threshold

15, at error severity. A new repo starts from a clean baseline. An existing repo takes a baseline
first (see Migration) and does not fix everything at once.

## When the hook reports a violation

1. Find the nesting and branching that drive the score: deep `if`/`elif`/`try` chains, nested
   loops, chains of boolean operators.
2. Extract the inner logic into a named helper. This usually makes the function easier to test too
   (CES-65).
3. Run the hook again to confirm that the function is under 15.

## Migration (existing repos)

`complexipy --snapshot-create` records the current violations as a baseline before the hook becomes
an error, so that existing complexity does not block unrelated pull requests. The shape is the
warn-then-error one of `file-size-guard` (CES-71).

## Suppression

`complexipy` has no suppression comment for a single function. A function that has to stay complex
(a generated parser, for example) is excluded through the file pattern of the hook, and the global
threshold stays at 15.
