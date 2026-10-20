## Directive

Importable library code logs through a logger. In Python, `print()`, `sys.stdout.write()` and
`sys.stderr.write()` are violations. In TypeScript and JavaScript (`.ts`, `.tsx`, `.js`), a call to
`console.log`, `info`, `debug`, `warn`, `error`, `trace`, `dir` or `table`, and
`process.stdout.write()` or `process.stderr.write()`, are violations. A warning by default.

## Why

These calls write unstructured text to stdout or stderr, with no level, timestamp or context, and a
library that prints pollutes the output of every program that imports it. Logging lets the
application decide where output goes.

## Exceptions

The entry points of a command line program and the tests write there on purpose. Another entry
point goes in the `extend-ignores` of the rule.

### Python

The rule skips:

- `__main__.py`, `cli.py` and every file under a `cli/` directory.
- `conftest.py` and every file under `tests/`.
- A write inside an `if __name__ == "__main__":` block.

### TypeScript and JavaScript

The rule skips:

- `cli.*` and every file under a `cli/` or `bin/` directory.
- `*.test.*`, `*.spec.*` and every file under `__tests__/` or `tests/`.
- A write inside an `if (import.meta.main)` or `if (require.main === module)` block.

## Do

### Python

Log through the house `get_logger` (`botica refs show structured-logging`):

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
