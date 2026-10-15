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

```ts
logger.info("running");
```

## Don't

```ts
export function run(): void {
  console.log("running");
}
```

## Suppression

Silence one line with `# botica: ignore[log-no-print]` or `# noqa: BTLOG002` in Python, and with
`// botica: ignore[log-no-print]` or `// noqa: BTLOG002` in TypeScript.
