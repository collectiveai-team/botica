---
role: tests
applies:
  paths: []
  tiers: [low, medium, high]
stages: [local, ci]
blocking: P1
max-findings: 10
---

## Invariants

- Each behaviour of the code has a test that fails when the behaviour is wrong.
- A test is deleted, skipped or loosened only with the behaviour it tested, or when another test
  asserts the same thing.
- A threshold of coverage or of mutation, and a check of CI or of a hook, is not lowered or
  silenced.

## Check always

- A deleted, skipped or loosened test, and the reason the diff gives for it.
- A lowered threshold, a check removed from CI or from a hook, `|| true` or `continue-on-error` on
  a step that runs tests.
- Each new or changed behaviour has a test that reaches it through the interface the rest of the
  code calls and asserts on a value that a wrong behaviour changes.

## Do not report

- What `botica lint` reports under the rules of the family `testing`.
- A missing test for code that only moved, with no change of behaviour.
- The style of a test that protects its behaviour.

<!-- botica:covered -->
<!-- /botica:covered -->

## Evidence

Each finding names the behaviour and the test, with file and line, and what in the test or the
diff shows that a wrong behaviour would pass.
