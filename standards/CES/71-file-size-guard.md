---
id: CES-71
title: Keep files small
---

## Directive

A source file of 400 lines or more is a warning and one of 700 lines or more is an error. Split a
module along a real seam before it crosses the hard limit.

## Why

The length of a file is a cheap proxy for too many responsibilities. The thresholds trigger
judgment, they do not replace it: a persistently large file is a design smell, and the fix is to
extract a cohesive unit, not to raise the threshold.

## Rules

- `file-size-guard`
