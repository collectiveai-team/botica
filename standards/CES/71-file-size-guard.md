---
id: CES-71
title: Keep files small
---

## Directive

A source file of 400 lines or more is a warning and one of 700 lines or more is an error. Split a
module along a real seam before it crosses the hard limit.

## Why

The length of a file is a proxy for the number of responsibilities it holds. A file that stays over
a threshold is split by extracting a cohesive unit.
