---
id: CES-122
title: A pull request carries one intent
---

## Directive

A pull request carries one intent, stated in one sentence in its title and description. A
reformatting sweep, a refactor that the intent does not need and the fix of something unrelated go
in pull requests of their own. A refactor that the intent needs first may share the pull request
when the description says so.

Track work in tickets. Group tickets into vertical slices that can be reviewed as end-to-end
behaviours. Integrate the reviewed slices into a delivery with one intent.

Implement tickets as local commits linked to their acceptance criteria. Review each slice's
accumulated diff between pinned base and head commits against its spec. Record the evidence.
Before publication, check the integrated delivery against the whole spec and the contracts
between slices. Choose review boundaries by behaviour, regardless of ticket, commit or line counts.

Publish one pull request per delivery by default. Record the reason for a split: earlier shipment,
separate approval, or independent delivery and rollback. Keep dependent local commits in the
delivery PR unless such a split is needed. A completed ticket may remain unpublished.

After three adversarial review rounds with a confirmed P0 or P1 still open, stop that slice and
escalate the unresolved finding or design decision. Preserve the implementation and its tests.
Retain the review evidence and mark the slice blocked. Discarding or rewriting the work requires
an explicit decision. Run the integration check even when every slice review passes.

## Why

A reviewer reads each line against the intent of the change. With two intents in one diff, the
reviewer has to sort the lines first, the review takes longer and misses more, and a revert takes
both out. Google's guide on small changes gives the same advice:
<https://google.github.io/eng-practices/review/developer/small-cls.html>.

This repository's standard-count check is separate from its ticket tracking.
`scripts/check_rule_change.py` permits one standard by default. A larger set requires repeated
`--acknowledged-standard` arguments naming exactly the touched set. ADR 0021 records the
authorized CES-122/CES-134 delivery.
