## Directive

Don't leave `console.log(...)` in library code: use the project's logger. Release 0.10.0 removed
this rule: its TypeScript check is part of `log-no-print`, which also reports the other methods of
`console` and `process.stdout.write()`.

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

The rule no longer runs, so there is nothing to suppress. `log-no-print` takes
`// botica: ignore[log-no-print]` or `// noqa: BTLOG002`.
