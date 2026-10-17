A test drives the logic of a service through a fake that implements the same Protocol as the
production adapter.

## What to read

- `UserRepository` is the port. The SQL adapter and the fake both implement it.
- `InMemoryUserRepository` is the fake. It stores users in a dict, so a test runs without I/O.
- The usage comment at the end shows a test that gives the fake to the service and asserts on the
  result through `get_by_email`.

## What to adapt

- `User` and `UserRepository` are one example. Write the Protocol of the port that your service
  depends on, and a fake that implements all of it.
- Put the fake beside your tests, for example in `tests/fakes/`.
