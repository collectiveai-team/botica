## Directive

A change keeps the existing, deliberate choices of the repository: its version pins, the configuration of its tools, its dependencies and its layout. A choice that a standard would make another way changes only in a migration that the pull request names and explains.

## Why

A pin or a setting of the configuration records a decision, and the history of the file gives its reason. Overwriting it to match a default loses that context and can break the build. A reviewer reads a change against its description, so a rewrite that the description does not mention goes unreviewed.

## Flag

- A version pin, such as `requires-python` or a `rev` of a hook, changed to the house default with no reason given.
- A `pyproject.toml`, `prek.toml` or `tsconfig.json` replaced by a template instead of edited.
- A dependency or a tool swapped for the house one in a change about something else.

## Do not flag

- A file that did not exist and that the change adds.
- A migration that the title or the description names, with its reason.
- A pin that a tool bumps on its own schedule, such as the pull requests of Dependabot.

## Confirm

Before reporting, check the description and the history of the file for the reason of the old value.

## Do

```text
chore(python): move requires-python to 3.14

The last dependency that held us on 3.12 (pyarrow) ships wheels for 3.14 since 22.0.

-requires-python = ">=3.12"
+requires-python = ">=3.14"
```

## Don't

```text
feat(api): add the orders route

-requires-python = ">=3.12"
+requires-python = ">=3.14"
-line-length = 120
+line-length = 100
```

## Suppression

The rule judges a whole change, so only the configuration silences it: `ignore = ["respect-local-repo"]`, or the paths in its `extend-ignores`.
