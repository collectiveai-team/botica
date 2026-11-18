# Review-flow standard acknowledgment

Status: accepted
Date: 2026-11-18

## Decision

The user authorized retaining the CES-122 process change and the risk-review standard in the
same delivery PR. `scripts/number_standard.py CES` assigned the risk-review proposal CES-134
after checking the fetched destination's standard numbers. The delivery intent is to implement
risk-based agent review.

The `review-flow-standards` PR label supplies exactly these arguments to the contract job:

```sh
--acknowledged-standard CES/122 --acknowledged-standard CES/134
```

The acknowledged set must equal the standards touched by the change. A third standard fails.
`contract-change` continues to acknowledge files outside a new rule's contract; it does not
acknowledge multiple standards. Duplicate numbers and an unnumbered ready PR still fail.
Other PRs retain the one-standard default unless they receive an explicit exact-set acknowledgment.

Changes to `contract-change` or `review-flow-standards` rerun only the contract job,
in a separate concurrency group.
This acknowledgment permits one delivery boundary. It does not approve the standards' contents,
authorize a merge, activate automation or claim live-service validation.

## Evidence

`tests/test_standard_acknowledgment.py` exercises the public branch checker and CLI with exact,
missing and larger sets, duplicate numbers, unnumbered standards and independent rule-contract
acknowledgment. The workflow tests check the delivery-specific CI arguments.
