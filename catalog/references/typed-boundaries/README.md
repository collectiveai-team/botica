A function that crosses a boundary returns a typed structure and not a bare `dict`.

## What to read

- `Point` is a frozen `@dataclass`, the shape of a boundary that needs no validation. `origin`
  returns it.
- `NewUser` is a pydantic `BaseModel` of the domain, the shape of a boundary whose fields are
  validated. `normalize` takes one and returns one.

The rules report the two shapes of a bare `dict` at a boundary:

```python
def origin() -> dict:  # a `dict` return annotation
    return {"x": 0, "y": 0}  # a returned `{...}` literal
```

## What to adapt

- `Point` and `NewUser` are examples. Name the structure for each boundary of your code.
- Use a dataclass when no validation is needed and a `BaseModel` where it is.
- The input and the output of an API are request and response schemas, with their own names and
  settings: see the topic `api-schemas`.
