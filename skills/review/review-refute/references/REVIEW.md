---
role: refute
applies:
  paths: []
  tiers: [low, medium, high]
stages: [local, ci]
blocking: P1
max-findings: 50
---

## Check always

- Each finding of P0 and P1 gets a reproduction attempt, not only a reading.
- A finding refuted as `pre-existing` names the commit of the target where the line already was.

## Do not report

- New findings. This role refutes; it does not review the diff again.

## Evidence

Each refutation gives the command or the file and line that disproves the finding.
