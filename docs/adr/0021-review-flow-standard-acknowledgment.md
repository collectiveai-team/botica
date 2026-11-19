# Review-flow standard acknowledgment

Status: accepted
Date: 2026-11-18

## Decision

On 2026-11-18 the user authorized keeping CES-122 and the risk-review standard in one delivery PR.
`scripts/number_standard.py CES` assigned CES-134 to the risk-review proposal. The fetched
destination's highest standard number was CES-133. The delivery implements risk-based agent review.

The `review-flow-standards` PR label supplies exactly these arguments to the contract job:

```sh
--acknowledged-standard CES/122 --acknowledged-standard CES/134
```

`scripts/check_rule_change.py` requires the acknowledged set to equal the touched set.
It rejects a third standard under this label. It also rejects duplicate numbers and ready PRs
with an unnumbered standard. Files outside a new rule's contract require the separate
`contract-change` acknowledgment. The default remains one standard.

In `.github/workflows/ci.yml`, changes to either acknowledgment label rerun only the contract
job, in a concurrency group separate from the review run.

The authorization covers keeping both standards in this delivery PR. Approval of their contents
and permission to merge remain separate decisions. Automation remains disabled; live-service
validation remains unverified.

## Evidence

`tests/test_standard_acknowledgment.py` exercises the public branch checker and CLI with exact,
missing and larger sets, duplicate numbers, unnumbered standards and independent rule-contract
acknowledgment. The workflow tests check the delivery-specific CI arguments.
