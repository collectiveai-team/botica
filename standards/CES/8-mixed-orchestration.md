---
id: CES-8
title: Separate orchestration from logic
---

## Directive

Orchestration (the order of steps, control flow, coordinating calls) does not share a function
with business logic and I/O. Keep "what happens in what order" thin and readable, and push "how
each step works" into named units that it calls.

## Why

A function that fetches, branches, computes, logs and writes at once has no seam for a test: the
logic cannot run without the I/O, and the flow cannot be followed without reading the details. With
the orchestration apart, it reads as a list of steps and each step is tested alone, through its
interface (CES-65).
