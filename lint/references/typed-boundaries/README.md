A function that crosses a boundary returns a typed structure and not a bare `dict`.

## What to read

- `Point` is a `@dataclass`, the shape of an internal boundary. `origin` returns it.
- `CreateUser` is a pydantic `BaseModel`, the shape where the input or the output is validated.
  `normalize` takes one and returns one.
- The comment at the end shows a `dict` return annotation and a `{...}` literal that is returned,
  which the rules report.

## What to adapt

- `Point` and `CreateUser` are examples. Name the structure for each boundary of your code.
- Use a dataclass when no validation is needed and a `BaseModel` where it is, such as the input and
  the output of an API.
