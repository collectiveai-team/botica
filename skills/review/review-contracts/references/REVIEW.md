---
role: contracts
applies:
  paths: []
  tiers: [medium, high]
stages: [local, ci]
blocking: P1
max-findings: 8
---

## Contracts of this repository

<!-- List what others consume: the CLI and its flags, the format of botica.toml, the output
formats, the files a recipe installs. -->

## Check always

- A default, a unit or an error code that changed with the same shape.
- A migration that the old code cannot run against.

## Do not report

- What the compatibility jobs compare: removed paths or fields, new required fields, changed types.

## Evidence

Each finding names the contract, the consumer and the input on which the consumer gets a different
result.
