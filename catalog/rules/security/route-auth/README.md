## Directive

Each route of a FastAPI application gets its caller from one of the dependencies that the option
`dependencies` names, or the option `public` lists it. A route is a function decorated with
`@<object>.get`, `post`, `put`, `patch`, `delete`, `options`, `head`, `trace`, `api_route` or
`websocket`. It is guarded when one of these holds `Depends(<dependency>)` or
`Security(<dependency>)`:

- a parameter of the function, in its default or in its `Annotated[...]` type;
- the `dependencies` of the decorator;
- the `APIRouter(...)` or `FastAPI(...)` call that makes the object of the decorator, in the same
  file.

A parameter whose type is an alias of the module that holds a guard (`CurrentUser =
Annotated[User, Depends(get_current_user)]`), or a name listed in `dependencies`, also guards it.
The rule reads one file at a time, so a guard given in `include_router(..., dependencies=...)` in
another file is not seen: list that alias or put those routes in `extend-ignores`. An error by
default; without `dependencies` it reports nothing.

## Why

A route with no check of the caller answers anyone who can reach it, and nothing in its code shows
that the check is missing. The rule makes the missing dependency a finding of the run.

## Do

```toml
[rules.route-auth]
dependencies = ["get_current_user", "CurrentUser"]
public = ["health", "login"]
```

```python
@router.get("/orders")
async def list_orders(user: Annotated[User, Depends(get_current_user)]) -> list[Order]: ...


admin = APIRouter(prefix="/admin", dependencies=[Depends(get_current_user)])


@admin.delete("/orders/{order_id}")
async def delete_order(order_id: int) -> None: ...
```

## Don't

```python
@router.get("/orders")
async def list_orders(user_id: str = Header()) -> list[Order]: ...
```

## Options

- `dependencies` (strings, empty by default): the functions that verify the caller, and the
  `Annotated` aliases imported from other modules that carry one. A dotted name matches by its last
  part.
- `public` (strings, empty by default): the names of the route functions that need no caller.

## Suppression

A comment on the line of the decorator silences it: `# botica: ignore[route-auth]`. A file of public
routes goes in the `extend-ignores` of `[rules.route-auth]`.
