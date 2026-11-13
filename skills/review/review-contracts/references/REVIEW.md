---
role: contracts
applies:
  paths: []
  tiers: [medium, high]
stages: [local, ci]
blocking: P1
max-findings: 8
---

## Invariants

- A contract that another program consumes keeps its meaning, or the change moves its version and
  says what a consumer of the old version gets.
- A consumer that did not update can still read and run what the change writes, or the change
  says it cannot.
- A migration runs with the old code and the new code during a deploy.

## Check always

- A default, a unit, an order, an error code or an exit code that changed with the same shape.
- A flag, an environment variable or a key of a configuration file removed or renamed with no
  alias.
- A file format or an output format whose readers the change does not update.
- A migration that the old code cannot run against.

## Do not report

- What the compatibility jobs compare: removed paths or fields, new required fields, changed types.

<!-- botica:covered -->
<!-- /botica:covered -->

## Evidence

Each finding names the contract, the consumer and the input on which the consumer gets a different
result.
