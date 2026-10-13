# no-console-log · `BTLOG003`

**Standard:** none yet · **Family:** logging · **Languages:** TypeScript

## Directive

Don't leave `console.log(...)` in library code: use the project's logger. A warning by default.
A call inside a `catch` clause is left alone, and so are the files named `*.test.ts`.

## Why

`console.log` writes unstructured text to the standard output, with no level and no context, and
it cannot be routed or silenced by the application that uses the library.

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

Silence one line with `// botica: ignore[no-console-log]` or `// noqa: BTLOG003`.
