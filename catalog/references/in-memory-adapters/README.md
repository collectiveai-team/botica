A test drives the logic of a service through a fake that implements the same Protocol as the
production adapter.

## What to read

- `UserRepository` is the port. The SQL adapter and the fake both implement it, and `UserService`
  depends on the port only.
- `InMemoryUserRepository` is the fake. It implements the whole port with a dict, so a test runs
  without I/O.
- The tests give the fake to the service and assert on the result through the interface:
  `register` and `get_by_email`, not the dict of the fake.

## What to adapt

- `User`, `UserRepository` and `UserService` are one example. Write the Protocol of the port that
  your service depends on, and a fake that implements all of it.
- Put the fake beside your tests, in `tests/fakes/`.
