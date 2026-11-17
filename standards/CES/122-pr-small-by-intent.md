---
id: CES-122
title: A pull request carries one intent
---

## Directive

A pull request carries one intent, stated in one sentence in its title and description. A
reformatting sweep, a refactor that the intent does not need and the fix of something unrelated go
in pull requests of their own. A refactor that the intent needs first may share the pull request
when the description says so.

Tickets track work; vertical slices define end-to-end behaviours to review; a delivery integrates
those slices for one intent. Several tickets may implement one slice, and several slices may
share a delivery. Completing a ticket does not require a pull request.

Implement tickets as local commits with links to their acceptance criteria. Review the accumulated
diff of each vertical slice against a pinned base and head, with its spec and evidence. Before
publication, check the integrated change against the whole spec, including the contracts between
slices. A ticket count, a commit count or a line count does not determine the review boundary.

Publish one pull request per delivery by default. Split it when a part must ship earlier, needs a
separate approval boundary, or can be delivered and reverted independently; record the reason.
A dependency between local commits does not require a stack of pull requests.

After three adversarial review rounds with a confirmed P0 or P1 still open, stop that slice and
escalate the unresolved finding or design decision. Preserve its code, tests and review evidence.
Do not publish it as ready; discarding or rewriting the work requires an explicit decision.
Passing slice reviews does not replace the integration check.

## Why

A reviewer reads each line against the intent of the change. With two intents in one diff, the
reviewer has to sort the lines first, the review takes longer and misses more, and a revert takes
both out. Google's guide on small changes gives the same advice:
<https://google.github.io/eng-practices/review/developer/small-cls.html>.

This repository also limits a pull request to one standard:
`scripts/check_rule_change.py` fails one that touches two. That contract does not require a pull
request for each ticket.
