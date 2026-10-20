---
id: CES-122
title: A pull request carries one intent
---

## Directive

A pull request carries one intent, which one sentence with no "and" states in its title. A
reformatting sweep, a refactor that the intent does not need and the fix of something unrelated go
in pull requests of their own. A refactor that the intent needs first may share the pull request
when the description says so.

The size of a change is not the measure: a rename across a hundred files has one intent, and a
change of ten lines can have two.

## Why

A reviewer reads each line against the intent of the change. With two intents in one diff, the
reviewer has to sort the lines first, the review takes longer and misses more, and a revert takes
both out. Google's guide on small changes gives the same advice:
<https://google.github.io/eng-practices/review/developer/small-cls.html>.
