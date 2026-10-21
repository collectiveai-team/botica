The request and response models of a public API set `model_config = ConfigDict(extra="forbid")`
and live under `api/<version>/schemas/requests/` and `api/<version>/schemas/responses/`.

## What to read

- `CreateUserRequest` is an inbound model. A payload with a key that the model does not declare
  fails validation with a 422.
- `UserResponse` is an outbound model with the same setting. FastAPI already limits the output to
  the fields of the response model, so on a response `forbid` does something else: a handler that
  returns a `dict` with a key the model does not declare fails with a 500, and
  `model_validate(row, from_attributes=True)` ignores the setting and drops the extra attributes.
- The router of `api/<version>/routers/` imports the schemas and defines none. It builds the
  response with the constructor, one field at a time.

## What to adapt

- `CreateUserRequest` and `UserResponse` are one example. Set `model_config` on every model of the
  schema modules of the API.
- The models of the domain and the models internal to a service do not take the setting.
- A field typed `EmailStr` needs the `email` extra of pydantic (`pydantic[email]`); the example uses
  `str` with a length.
