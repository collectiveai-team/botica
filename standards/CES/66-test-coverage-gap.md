---
id: CES-66
title: Coverage gaps are a signal
---

## Directive

An uncovered branch is a question and not a number. Every coverage gap is one of three things: a
missing test, dead code, or a seam that cannot be tested and wants refactoring. The gap is
investigated to find which one, and `# pragma: no cover` is not added to raise a percentage.

## Why

A test that asserts nothing and an excluded file both raise the percentage, so the number alone
says nothing about the tests. The uncovered line is the signal: it leads to a test that checks
behaviour, or to the deletion of code that nothing needs.
