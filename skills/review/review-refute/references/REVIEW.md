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

- Each finding of `P0` and `P1` gets an attempt against the files of the target or of `HEAD` (a
  command of `git` on the machine of the author, a read of the file in CI), not only a reading of
  its text.
- A finding refuted as `pre-existing` names the commit of the target where its lines already were,
  and says that no line of the diff changes what reaches them.
- A finding about something absent or removed (a missing criterion, a behaviour with no test, a
  deleted test or check) is judged by whether the diff should have added it, never as
  `pre-existing`.
- A finding that only a run of code could reproduce stays `confirmed`, and its `how_to_verify`
  has a step that a person runs.

## Do not report

- New findings. This role refutes; it does not review the diff again.

## Evidence

Each refutation gives the command that the role ran or the file and line that disproves the
finding. `max-findings` is the number of findings that the role judges; `refute.json` keeps every
finding of the inputs.
