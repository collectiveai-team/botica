The request and response models of a public API set `model_config = ConfigDict(extra="forbid")`.
A field that the model does not declare is rejected with a 422.

## What to read

- `CreateUserRequest` is an inbound model. A payload with a key that the model does not declare
  fails validation.
- `UserResponse` is an outbound model with the same setting, so the contract of the response is the
  fields that the model declares.
- The comment at the end shows a model with no `model_config`, which the rule reports.

## What to adapt

- `CreateUserRequest` and `UserResponse` are one example. Set `model_config` on every model of the
  schema modules of the API (`api/schemas/requests/` and `api/schemas/responses/`).
- The models of the domain and the models internal to a service do not take the setting.
