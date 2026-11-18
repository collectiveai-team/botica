---
role: spec
applies:
  paths: []
  tiers: [low, medium, high]
stages: [local]
blocking: P1
max-findings: 10
covered: {}
---

## Check

Compare each acceptance criterion with code and observable evidence in the diff.
Report missing behaviour and changes outside the stated intent. Apply explicitly approved
scope changes before comparing with a historical spec.

Quote the requirement for each finding and identify the affected file and line, or the
missing implementation. Record unavailable requirements under `not_checked`.

<!-- botica:covered -->
<!-- /botica:covered -->
