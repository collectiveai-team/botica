## Directive

No cast or type assertion gives the type checker a type that nothing checks. The rule reports:

| Language | Reported |
|---|---|
| Python | a call to `cast` of `typing` or `typing_extensions`: `cast(T, x)` after `from typing import cast`, also when the import is inside a function or under `if TYPE_CHECKING:`; `c(T, x)` after `from typing import cast as c`; `typing.cast(T, x)`; and `t.cast(T, x)` after `import typing as t` |
| TypeScript, TSX | an assertion to `any`: `x as any`, `<any>x`, also inside a double assertion (`x as any as T`); and an assertion through `unknown` or `never`: `x as unknown as T`, `(x as unknown) as T`, `<T>(<unknown>x)` |

A method `cast` of another object (`pl.col("a").cast(pl.Int64)`), the `cast` of SQLAlchemy or of the
project, a single assertion `x as T`, which TypeScript rejects when the two types do not overlap,
and the non-null assertion `x!` are left alone. `any` in an annotation (`x: any`, `Promise<any>`)
is not a cast: `noExplicitAny` of Biome reports it, and it is an error in the `biome.json` of
`base-node`. Biome also reports `x as any` and `<any>x`, so a project with Biome sees those twice.
A `cast` that a module of the project imports from `typing` and exports again is not seen. An
error by default.

## Why

`cast(User, value)`, `value as any` and `value as unknown as User` tell the checker that the value has
a type, and the program does not check it when it runs: the checker stops reporting the error, and
the value that caused it stays. Unlike `# pyrefly: ignore[...]` or `// @ts-expect-error`, a cast
leaves no comment that a reader or a tool can find. pyrefly reports a `cast` only when the two types
cannot overlap (`invalid-cast`, off by default).

## Do

```python
def is_user(value: object) -> TypeIs[User]:
    return isinstance(value, User)


def load(payload: dict[str, object]) -> User:
    user = payload["user"]
    if not is_user(user):
        raise TypeError("payload['user'] is not a User")
    return user
```

```ts
export async function fetchUser(id: string): Promise<User> {
  const body: unknown = await (await fetch(`/users/${id}`)).json();
  return UserSchema.parse(body);
}
```

## Don't

```python
def load(payload: dict[str, object]) -> User:
    return cast(User, payload["user"])
```

```ts
export async function fetchUser(id: string): Promise<User> {
  const body: unknown = await (await fetch(`/users/${id}`)).json();
  return body as unknown as User;
}
```

## Suppression

A cast whose type the code knows from somewhere the checker cannot see, such as a library with
wrong stubs, stays with the reason on its line: in Python `# botica: ignore[no-type-cast] -- <reason>`
or `# noqa: BTTOOL004 -- <reason>`, in TypeScript `// botica: ignore[no-type-cast] -- <reason>`.
ruff takes the `# noqa` form without a finding when `external` of its configuration lists `BT`, as
the section of ruff of `base-python` does.
