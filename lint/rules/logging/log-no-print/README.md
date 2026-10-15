## Directive

Don't use `print()` in Python or `console.log()` in TypeScript in importable library code. Emit
through the logger of the project: the house `get_logger` in Python
(`.agents/snippets/core/logger.py`), and the logger the project uses in TypeScript. A warning by
default.

## Why

`print()` and `console.log()` write unstructured text to stdout, with no level, timestamp or
context, and a library that prints pollutes the stdout of every program that imports it. Logging
lets the application decide where output goes.

## What is exempt

Python: entrypoints, where stdout is the product: `__main__.py`, `cli.py`, anything under a `cli/`
directory, `conftest.py`, anything under `tests/`, and any `print()` inside an
`if __name__ == "__main__":` guard.

TypeScript: a call inside a `catch` clause, and the files named `*.test.ts`.

## Do

```python
log = get_logger(__name__)


def fetch(key):
    log.info("cache miss", key=key)
```

```ts
export function run(): void {
  logger.info("running");
}
```

## Don't

```python
def fetch(key):
    print("cache miss", key)
```

```ts
export function run(): void {
  console.log("running");
}
```

## Suppression

Python takes a comment with the slug and a `noqa` with the code. TypeScript takes the comment with
the slug.

```python
print("cache miss", key)  # botica: ignore[log-no-print]
print("cache miss", key)  # noqa: BTLOG002 -- reason
```

```ts
console.log("running"); // botica: ignore[log-no-print]
```
