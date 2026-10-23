## Directive

No setting or directive turns off the type checker for a whole file, a whole project or a build.
The rule reports:

| Where | Reported |
|---|---|
| TypeScript and JavaScript (`.ts`, `.tsx`, `.js`, `next.config.*`) | a `// @ts-nocheck` or `/* @ts-nocheck */` comment, and `ignoreBuildErrors: true` (the `typescript` option of `next.config`) |
| `tsconfig.json`, `tsconfig.*.json`, `jsconfig.json` | `"strict": false`, and `false` for an option of the strict family: `noImplicitAny`, `noImplicitThis`, `strictNullChecks`, `strictFunctionTypes`, `strictBindCallApply`, `strictPropertyInitialization`, `strictBuiltinIteratorReturn`, `useUnknownInCatchVariables`, `alwaysStrict` |
| Python | a `# pyrefly: ignore-errors`, `# mypy: ignore-errors` or `# pyre-ignore-all-errors` comment |

A comment that silences one line (`// @ts-expect-error`, `# pyrefly: ignore[<code>]`) and
`# pyrefly: ignore-errors[<code>]`, which silences one kind of error, are left alone. An error by
default.

## Why

An agent whose build fails on types can make it pass by turning the checker off, and the errors
stay in the code with nothing that reports them. Next.js documents `ignoreBuildErrors` as the way
to "dangerously produce production code even when your application has errors". Since
TypeScript 6.0 `strict` is `true` by default, so `"strict": false` is a choice someone wrote.

## Do

```ts
// @ts-expect-error the types of the library return `any[]`; see its issue 812
const rows: Row[] = client.rows();
```

```json
{ "compilerOptions": { "strict": true } }
```

## Don't

```ts
// @ts-nocheck
const rows: Row[] = client.rows();
```

```js
const config = { typescript: { ignoreBuildErrors: true } };
```

```python
# pyrefly: ignore-errors
```

## Suppression

In TypeScript and JavaScript, `// botica: ignore[no-disabled-typecheck]` on the line; in Python,
`# botica: ignore[no-disabled-typecheck]` or `# noqa: BTTOOL001 -- <reason>`. JSON has no comments
that botica reads: a `tsconfig` that has to keep an option off goes in the `extend-ignores` of
`[rules.no-disabled-typecheck]`.

A repository where the checker is off somewhere turns it back on in its own pull request: remove
the directive or the setting, run the checker (`pyrefly check`, `tsc --noEmit`), and fix the errors
or silence each on its line with the reason (`# pyrefly: ignore[<code>]`,
`// @ts-expect-error <reason>`). When the errors are too many for one pull request, keep the file in
the `extend-ignores` of `[rules.no-disabled-typecheck]` and take it out file by file. A `baseline`
file of pyrefly holds the existing errors only with `baseline-matching-mode = "column-ordered"`;
with the default mode, one entry hides every new error of the same kind at the same column.
