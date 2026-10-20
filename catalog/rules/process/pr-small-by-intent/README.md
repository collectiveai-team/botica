## Directive

A pull request carries one intent, which one sentence with no "and" states. A reformatting sweep, a refactor that the intent does not need and the fix of something unrelated each go in a pull request of their own.

## Why

A reviewer reads a change against its intent. When two intents share a diff, each line has to be sorted into one of them first, the review takes longer and misses more, and a revert takes both out. Google's guide on small changes says the same: <https://google.github.io/eng-practices/review/developer/small-cls.html>.

## Flag

- A diff that reformats or renames code that the intent does not touch.
- A refactor that the intent does not need, next to a feature or a fix.
- A fix of a bug that the title and the description do not name.
- A title or a description that needs "and" to say what the change does.

## Do not flag

- A rename or a migration that touches many files with one intent.
- A refactor that the intent needs first, in the same pull request, when the description says so.
- The tests, the documentation and the configuration that the intent changes.

## Confirm

Read the title and the description before the diff, and check each file of the diff against the intent they state.

## Do

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
