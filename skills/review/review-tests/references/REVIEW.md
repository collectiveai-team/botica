---
role: tests
applies:
  paths: []
  tiers: [low, medium, high]
stages: [local, ci]
blocking: P1
max-findings: 10
---

## Check always

- A deleted, skipped or loosened test, and why the diff says it goes.
- A lowered threshold of coverage or mutation, or a check removed from CI or a hook.
- Each new behaviour has a test that fails when the behaviour is wrong.

## Do not report

- What `test-smell-review` or a scanner it runs already reported.
- What `botica lint` reports under BTTEST00x.
- A missing test for code that only moved, with no change of behaviour.

## Evidence

Each finding names the behaviour and the test, and says whether breaking the code by hand made the
test fail.
