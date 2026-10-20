## Directive

A comment says only what the code cannot say through its names and its structure: the reason for a choice that looks wrong, an invariant, a workaround with the link to its cause, or the docstring of a public interface.

## Why

A comment that repeats the code is a second copy of it, and the two drift apart when the code changes. A reader then trusts the comment and misreads the code.

## Flag

- A comment that says what the next line does: `# increment the counter`.
- A comment that names a section of a file: `# --- helpers ---`.
- Code commented out.
- A comment that tells the history of a change: `# changed by X to fix Y`.

## Do not flag

- A docstring of a public function, class or module.
- A comment that a tool reads: a suppression, a type ignore, a license header, a shebang, a
  `MARK:` line that `botica recipe` and editors use to find a section.
- An option of a template left commented out under a comment that says how to turn it on.
- A comment that gives the reason for a choice, an invariant or a link to a bug.
- A generated file, such as a lockfile or a schema.

## Confirm

Before reporting a comment as a repetition, read the code it sits on and check that the code alone says the same.

## Do

```python
# The API rejects a page size above 100, though its documentation says 500 (issue #412).
PAGE_SIZE = 100
```

## Don't

```python
# set the page size to 100
PAGE_SIZE = 100
```

## Suppression

A file takes `botica: ignore-file[minimal-comments] -- <reason>` in a comment, such as a teaching example. A folder goes in the `extend-ignores` of the rule.
