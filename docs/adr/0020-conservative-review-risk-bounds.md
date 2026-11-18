# Conservative review risk bounds

Status: proposed
Date: 2026-11-17

## Decision

For text changes whose unsupported detector inputs are exclusively Rust source paths,
`review tier` reports an explicit lower and upper bound. The lower bound retains area
floors and detected facts. The upper bound also includes the medium floor of a possible
new dependency or endpoint. Critical area floors remain high.

The current risk policy raises risk for these two detector facts only. Their presence
cannot raise an area floor above medium. This makes medium a conservative upper bound
when the area floor is low. `complete` means a risk bound is available, not that the
unsupported detectors established absence of either fact. Detector evidence stays
unsupported, and possible signals never enter the positive facts.

`review plan` conservatively selects signal- and tier-filtered roles when bounded
uncertainty exists. Paths and execution stages still apply. This can run extra roles.

## Limits

The supported pilot boundary is text Rust source, documentation, known policy files
and the existing npm detector. Other unsupported inputs, non-text changes, failed
integration or unreadable policy retain incomplete status. Python ancestry scenarios
retain their positive endpoint evidence without gaining a complete risk assessment.

Size is informational. Only critical areas require human review. A technical failure
blocks execution or automation without creating a human-review requirement.

## Evidence

`crates/botica-cli/tests/review_tier.rs` tests the Rust bound through real Git history.
The source ancestry tests preserve the Python authentication criss-cross regression.
