# Conservative review risk bounds

Status: proposed
Date: 2026-11-17

## Decision

For text changes whose unsupported inputs belong to the dependency/endpoint boundary
or npm manifest detectors,
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

Amended 2026-11-18: the initial Rust-only boundary unnecessarily blocked CI configuration
and the Go/Node mutation recipes. The same policy proof applies to text in any language
and to uncertain npm constraints. Only the two named v1 detectors receive this bound;
a new detector needs its own justified bound. Non-text changes, unreadable files,
failed integration and unreadable policy retain incomplete status.

The Python authentication criss-cross scenario now has a complete high policy bound.
Its high critical evidence, every best common ancestor and unsupported detector
evidence remain visible. This supersedes its earlier incomplete/null result without
claiming exact detector certainty. An uncertain comparison never gains a low upper bound.

Size is informational. Only critical areas require human review. A technical failure
blocks execution or automation without creating a human-review requirement.

## Evidence

`crates/botica-cli/tests/review_tier.rs` tests the Rust bound through real Git history.
The source ancestry tests preserve the Python authentication criss-cross regression.
