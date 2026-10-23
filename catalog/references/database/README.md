The package `database` holds the SQLModel tables, the repositories that implement the ports of
`core`, the Alembic migrations, the engine and the session, and its settings. The schema changes
through a migration, the migrations run as a step of their own before the API starts, and the API
gets models of `core` from the repository and builds its responses from them.

## What to read

- `UserRepository` of `app/core/users.py` is the port, and `User` the model of the domain. `core`
  imports no `sqlmodel`, `sqlalchemy` or `alembic`.
- `UserRow` of `app/database/models/users.py` is the table. `app/database/models/__init__.py`
  imports every table, so importing `app.database.models` registers them all on
  `SQLModel.metadata`. A table that is not imported there is missing from the metadata, and
  `alembic revision --autogenerate` writes a migration that drops it or leaves it out.
- `SqlUserRepository` implements the port over a `Session` and turns each row into a `User`. A row
  does not leave `app.database`.
- `env.py` compares the migrations with `SQLModel.metadata`, reads the URL from `get_settings()` of
  `app/database/settings.py`, and uses the connection of the caller when
  `config.attributes["connection"]` holds one.
- `migration_config` names the migrations as `app.database:migrations`, a resource of the package,
  so they are found in an installed wheel as well as in a checkout. `upgrade_to_head` applies the
  migrations that the database does not have yet.
- `app/entrypoints/migrate.py` is the step of the deployment that runs `upgrade_to_head`, once,
  before the processes of the API start: a job, an init container or a release command (examples
  below). The lifespan of the API applies no migration: with several replicas each one would run
  the same DDL at the same time.
- `get_engine` builds the engine once per process; `get_user_repository` gives each request a
  repository over a session of its own, and is the dependency of the router.
- The router takes the port, not the session, and builds `UserResponse` from the `User` one field
  at a time with `UserResponse.of`. A response that sets `from_attributes=True` and reads a row
  would ignore `extra="forbid"` and drop the attributes it does not declare (topic `api-schemas`).
- The tests of `code/tests/` build a SQLite database in memory with the migrations, not with
  `SQLModel.metadata.create_all()`, and check that `compare_metadata` finds nothing for
  autogenerate to add. `StaticPool` keeps one connection for the test: each new connection to
  `sqlite://` opens a new, empty database.

## What to adapt

- `User`, `UserRow` and the endpoints are one example. Write one table module per aggregate under
  `models/`, import it in `models/__init__.py`, and one repository per port under `repositories/`.
- Put the configuration of the command `alembic` in `pyproject.toml`, so that
  `alembic revision --autogenerate -m "..."`, `alembic check` and `alembic upgrade head` work from
  the root of the project:

  ```toml
  [tool.alembic]
  script_location = "app.database:migrations"
  ```

- Read each candidate migration before committing it: autogenerate does not see a renamed table or
  column, and writes a drop and an add.
- Run `alembic check` in CI, against a database migrated to head, to fail a change to a table that
  has no migration.
- SQLite runs the migrations of the example and needs no server. It does not have every type and
  `ALTER` of PostgreSQL, so a project usually also runs its migrations against the engine of
  production, such as PostgreSQL in a container of the CI; nothing requires it.
- `DatabaseSettings` reads `APP_DATABASE_URL`. A higher layer stacks it as a field of its own
  settings (CES-76).

## Migrations as a step of the deployment

The image of the API carries the migrations, and the deployment runs them once with the command of
the migration step, `python -m app.entrypoints.migrate`, before it starts or replaces the processes
of the API. With `[tool.alembic]` in `pyproject.toml`, `alembic upgrade head` does the same. When
the migration fails, the deployment stops and the running version keeps serving.

In Docker Compose, a service runs the migration and exits, and the API waits for it to finish
with success:

```yaml
services:
  migrate:
    image: ghcr.io/example/orders:${VERSION}
    command: ["python", "-m", "app.entrypoints.migrate"]
    environment:
      APP_DATABASE_URL: ${APP_DATABASE_URL}

  api:
    image: ghcr.io/example/orders:${VERSION}
    command: ["uvicorn", "app.entrypoints.api:app", "--host", "0.0.0.0", "--port", "8000"]
    environment:
      APP_DATABASE_URL: ${APP_DATABASE_URL}
    depends_on:
      migrate:
        condition: service_completed_successfully
```

In GitHub Actions, a job migrates the database of the environment, and the job that deploys needs
it, so it does not run when the migration fails:

```yaml
jobs:
  migrate:
    runs-on: ubuntu-latest
    environment: production
    steps:
      - uses: actions/checkout@v7
        with:
          persist-credentials: false
      - uses: astral-sh/setup-uv@v10.3.0
      - name: Bring the database to the last migration
        run: uv run --frozen python -m app.entrypoints.migrate
        env:
          APP_DATABASE_URL: ${{ secrets.APP_DATABASE_URL }}

  deploy:
    needs: migrate
    runs-on: ubuntu-latest
    environment: production
    steps:
      - name: Deploy the new version of the API
        run: ./deploy.sh
```

A migration that the running version cannot work with, such as dropping a column it still reads,
goes in two releases: the code that stops using the column first, the migration after.
