## Directive

The `requires-python` of a project is a suggestion that no hook or CI job checks. A new project starts at 3.14, and an existing pin stays as it is: a change moves it only when its description names the move and gives the reason.

## Why

Forcing one minor version fights the dependencies that do not support it yet, and the ecosystem of 3.14 still has gaps. A pin that a change rewrites to the default can drop the version that a dependency needs, and the reason for the old value is lost.

## Flag

- An existing `requires-python` changed in a pull request about something else.
- A pin moved to `>=3.14` with no reason in the description.

## Do not flag

- The `requires-python` of a new project.
- A move that the title or the description names, with its reason.

## Confirm

Before reporting, read the description for the reason of the move.

## Do

```text
chore(python): move requires-python to 3.14

pyarrow, the last dependency that held the project on 3.12, ships wheels for 3.14 since 22.0.
```

## Don't

```text
feat(api): add the orders route

-requires-python = ">=3.12"
+requires-python = ">=3.14"
```

## Suppression

The rule judges a whole change, so only the configuration silences it: `ignore = ["version-pin"]`, or the paths in its `extend-ignores`.
