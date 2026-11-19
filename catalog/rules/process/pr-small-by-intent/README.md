## Directive

A pull request carries one intent. Tickets track local work; vertical slices are reviewed as
accumulated diffs; one delivery PR is the default. Split for independent delivery, approval or
rollback. Ticket and line counts are informational. Unrelated changes get separate PRs. After three review rounds
with a confirmed P0/P1 left, preserve the work and escalate; do not publish it as ready.

## Why

A reviewer reads a change against its intent. When two intents share a diff, each line has to be sorted into one of them first, the review takes longer and misses more, and a revert takes both out. Google's guide on small changes says the same: <https://google.github.io/eng-practices/review/developer/small-cls.html>.

## Flag

- A diff that reformats or renames code that the intent does not touch.
- A refactor that the intent does not need, next to a feature or a fix.
- A fix of a bug that the title and the description do not name.
- A title or description that joins unrelated outcomes instead of naming one intent.

## Do not flag

- A rename or a migration that touches many files with one intent.
- A refactor that the intent needs first, in the same pull request, when the description says so.
- The tests, the documentation and the configuration that the intent changes.
- Several tickets or reviewed vertical slices that serve one delivery intent.
- Local dependent commits submitted together, without a pull request for each commit.
- A title with "and" that describes parts of one behaviour, such as saving and restoring a session.

## Confirm

Read the title, description and spec before the diff, and check each part against the intent they
state. Ticket count and the word "and" alone are not evidence of unrelated intents.

## Do

Implement tickets as local commits and review each slice's accumulated diff against pinned
commits, with its spec and acceptance evidence. Check the integrated change against the whole
spec before publication. State the delivery intent in one sentence in the title and description.

Record the reason when a part must ship earlier, needs a separate approval boundary, or can be
delivered and reverted independently. A reformatting sweep, an unnecessary refactor or an unrelated
fix belongs to a different intent.

When a slice is blocked after three rounds, preserve code, tests and evidence. Discarding or
rewriting requires an explicit decision.

```text
fix(lint): route-auth reads a dependency given by keyword

 crates/botica-core/src/route_auth.rs | 6 ++++--
 crates/botica-core/tests/route_auth.rs | 9 +++++++++
```

## Don't

```text
feat(lint): route-auth, and format the workspace, and fix the cache key

 crates/botica-core/src/route_auth.rs | 240 ++++++++++
 crates/botica-core/src/cache.rs      |   4 +-
 crates/*/src/**                      | 812 +++++------
```

## Suppression

The rule judges a whole change, so only the configuration silences it: `ignore = ["pr-small-by-intent"]`, or the paths in its `extend-ignores`.
