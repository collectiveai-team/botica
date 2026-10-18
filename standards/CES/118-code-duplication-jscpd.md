---
id: CES-118
title: Copy-paste duplication
---

## Directive

A block of code above the token threshold of [`jscpd`](https://github.com/kucherenko/jscpd) is not
duplicated, near-verbatim, elsewhere in the repo without an explicit exemption. The check is a
duplication percentage for the whole repo (`--threshold`), and the hook fails when it is exceeded.

## Why

Copy-pasted logic is a common failure of AI agents. An agent asked to add a similar feature copies
the nearest block and does not extract a shared helper, so a later fix to one copy does not reach
the other. Ruff, botica and import-linter do not detect this: a rule of botica matches one declared
pattern and cannot match "this block looks like that other block". `jscpd` is polyglot, so it also
covers the frontend, Docker and YAML files of a repo and not only Python.

## Threshold and scope

The threshold is `--threshold 1` (percent) and is calibrated per repo. `jscpd` respects
`.gitignore` by default, and v5 needs no `--gitignore` flag. A file that mirrors another on purpose,
such as a guide that repeats the instructions of a README, is exempted with `-i` or `--ignore` glob
patterns, and the global threshold does not go up to cover real duplication elsewhere.

`--threshold` alone is the pass or fail gate: the command exits non-zero only when the duplication
is above the percentage. `jscpd` v5 also has `--exit-code [<code>]`, which exits with that code
when any duplicate is found. It is not combined with `--threshold`: it fails on any clone, whatever
the percentage, and so the threshold has no effect.

## When the hook reports a violation

1. Find the shared logic behind the clone and extract it into a named function or module.
2. When the duplication is deliberate, exempt the path with `-i` or `--ignore` and write a comment
   that says why.
3. Run the hook again to confirm that the repo is under the threshold.

## Migration (existing repos)

Run `npx jscpd@5 . --threshold 1` once to take a baseline. Either fix the real duplication or
calibrate the threshold and the ignore list to the state that the repo has and understands. Write
down each deliberate exemption where it is made. The posture is the one of `file-size-guard`
(CES-71): flag, and do not exempt silently.

## Suppression

By path: `-i` or `--ignore` glob patterns in the `entry` of the hook. By code: `--ignore-pattern`
with a regex that skips specific tokens, such as a license header. The exemption is scoped to the
duplicate it covers, and the global threshold does not go up to hide it.
