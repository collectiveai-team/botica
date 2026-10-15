## Directive

Importable library code logs through a logger. A `print()` in Python or a `console.log()` in
TypeScript is a violation. A warning by default.

## Why

`print()` and `console.log()` write unstructured text to stdout, with no level, timestamp or
context, and a library that prints pollutes the stdout of every program that imports it. Logging
lets the application decide where output goes.

## What is exempt

### Python

The rule skips:

- `__main__.py`, `cli.py` and every file under a `cli/` directory, because stdout is their output.
- `conftest.py` and every file under `tests/`.
- A `print()` inside an `if __name__ == "__main__":` block.

### TypeScript

The rule skips:

- A `console.log()` inside a `catch` clause.
- Files named `*.test.ts`.

## Do

### Python

Log through the house `get_logger` (`.agents/snippets/core/logger.py`):

```python
log = get_logger(__name__)


def fetch(key):
    log.info("cache miss", key=key)
```

### TypeScript

Log through the logger of the project:

```ts
export function run(): void {
  logger.info("running");
}
```

## Don't

### Python

```python
def fetch(key):
    print("cache miss", key)
```

### TypeScript

```ts
export function run(): void {
  console.log("running");
}
```

## Suppression

### Python

A comment on the line silences it. There are two forms: `# botica: ignore[<slug>]` names the rule by
its slug, and `# noqa: <code> -- <reason>` names it by its code, with the reason after `--`.

```python
print("cache miss", key)  # botica: ignore[log-no-print]
print("cache miss", key)  # noqa: BTLOG002 -- reason
```

### TypeScript

One comment silences one line. It names the rule by its slug.

```ts
console.log("running"); // botica: ignore[log-no-print]
```
