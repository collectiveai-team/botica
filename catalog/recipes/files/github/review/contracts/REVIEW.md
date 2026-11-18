---
role: contracts
applies:
  paths: []
  tiers: [medium, high]
stages: [local, ci]
blocking: P1
max-findings: 8
covered: {}
---

## Check

Inspect changes to APIs, file formats, CLI arguments, environment variables and migrations.
Follow the value to an actual consumer. Check changes of meaning even when a schema's shape
stays the same.

Name the consumer and a concrete input that changes behaviour. Check compatibility evidence
before reporting a break. Omit only findings covered by declared checks that actually run.

<!-- botica:covered -->
<!-- /botica:covered -->
