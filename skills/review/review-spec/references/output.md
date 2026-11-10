# Output of a review role

Every `review-<role>` skill writes the same JSON, so that `review-refute`, `review-panel` and later
`botica lint --reviewed` read one format. This file is copied into each role skill, and
`tests/test_skill_catalog.py` checks that the copies match.

## Posture

Review to find what is wrong, not to approve. Report only what the diff introduced or made
reachable. Report nothing that a deterministic check of the repository already reports: `botica
lint`, a hook of prek, a job of CI. Read the `## Do not report` section of the role file before
writing a finding.

## File

`.tmp/review/<short-head-sha>/<role>.json`:

```json
{
  "role": "security",
  "head": "2e12962",
  "base": "main",
  "verdict": "FAIL",
  "findings": [
    {
      "id": "security-1",
      "severity": "P1",
      "path": "crates/botica-cli/src/server.rs",
      "line": 42,
      "title": "The token is compared with ==",
      "evidence": "What in the code shows it, and the smallest case that breaks it.",
      "reproduced": "yes | no | not-tried",
      "how_to_verify": ["cargo test -p botica-cli server::token"],
      "fix": "One sentence; no patch."
    }
  ],
  "checked": ["what this role reviewed"],
  "not_checked": ["what it did not review, and why"],
  "commands": [{"run": "cargo test -p botica-cli", "result": "pass"}]
}
```

## Verdict

- `PASS`: no finding at or above the blocking severity of the role file (P1 when it says none).
- `FAIL`: at least one finding at or above it.
- `INCONCLUSIVE`: the role could not do its review, for example no issue for `spec` or a test
  suite that does not run. `not_checked` says why.

## Severity

- `P0`: breaks production, loses data or opens a security hole on the normal path.
- `P1`: wrong behaviour that a user or a caller reaches.
- `P2`: wrong on an edge case, or a gap that makes a later bug likely.
- `P3`: worth knowing; never blocks.

At most `max-findings` findings (10 when the role file says none), the most severe first.
