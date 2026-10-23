A test drives the logic of a service through a fake that implements the same Protocol as the
production adapter.

## What to read

- `UserRepository` is the port. The SQL adapter and the fake both implement it, and `UserService`
  depends on the port only.
- `add` takes the email and returns the `User` with the id that the store gave it, as a database
  does with its primary key. A port that asks the store for the next id and stores the user in a
  second call lets two registrations that run at the same time take the same id.
- `InMemoryUserRepository` is the fake. It implements the whole port with a dict and numbers the
  users itself, so a test runs without I/O.
- The tests give the fake to the service and assert on the result through the interface:
  `register` and `get_by_email`, not the dict of the fake.

## What to adapt

- `User`, `UserRepository` and `UserService` are one example. Write the Protocol of the port that
  your service depends on, and a fake that implements all of it.
- Put the fake beside your tests, in `tests/fakes/`.
- The topic `database` has the SQL adapter of the same port: `botica refs show database`.
