A `compose.yaml` runs the application with its database, on the machine of a developer and on a
single host. `compose.override.yaml` adds what only local development needs, and Compose reads it
by itself when it finds it beside `compose.yaml`. The topic `compose-traefik` has the file for a
host that serves the application on the internet: `botica refs show compose-traefik`.

## What to read

- Each image has a tag: `postgres:18.6`, `adminer:6.1.2`. The image of the application takes its
  tag from `APP_TAG`, and `dev` when it is unset; the override builds that image from the
  `Dockerfile`.
- `depends_on` with `condition: service_healthy` starts `app` after the healthcheck of `db`
  passes. Without the condition, Compose waits only until the container of `db` runs, before
  Postgres accepts connections. `restart: true` restarts `app` when Compose restarts `db`.
- The healthcheck of `db` is `pg_isready`. The `$$` writes a `$` that the shell of the container
  expands, so Compose does not read `POSTGRES_USER` from the environment of the host.
- The healthcheck of `app` calls an endpoint of the application with the Python of the image, which
  has no `curl` in a slim image.
- The file holds no secret. The password of Postgres is the file `secrets/db_password.txt`, which
  the image reads through `POSTGRES_PASSWORD_FILE`. The application gets the URL of the database as
  the file `/run/secrets/app_database_url`. Unlike a variable of `environment`, a secret is not in
  the output of `docker inspect`.
- `env_file: .env` passes the variables of the application that are not secret, such as
  `APP_DEBUG`. `.env` is not committed.
- The data of Postgres is in the named volume `db-data`, which `docker compose down` keeps and
  `docker compose down --volumes` removes. Since Postgres 18 the image keeps its data under
  `/var/lib/postgresql`, not under `/var/lib/postgresql/data`.
- `adminer` has `profiles: [tools]`: `docker compose up` leaves it out, and
  `docker compose --profile tools up` starts it. It listens on 127.0.0.1 only.
- The override publishes the port of `app` on 127.0.0.1, so the application is not reachable from
  the network. `develop.watch` copies `src/` into the container and restarts it on each change, and
  builds the image again when `uv.lock` changes: `docker compose up --watch`.
- Each file passes `docker compose config` alone, as the hook `docker-compose-config` of CES-114
  checks it: the override touches only `app`, which has a `build`.

## What to adapt

- Replace `ghcr.io/acme/app` with the image that the workflow of the recipe `ci-docker` pushes,
  `ghcr.io/<owner>/<repo>`, and `/health` and the port 8000 with those of your application.
- Create the secret files and keep them out of git, with these lines in `.gitignore`:

  ```gitignore
  .env
  secrets/
  ```

  `secrets/db_password.txt` holds the password, and `secrets/app_database_url.txt` the URL with the
  same password, such as `postgresql+psycopg://app:<password>@db:5432/app`.
- Read the secret in the settings of the package with `secrets_dir="/run/secrets"` in the
  `SettingsConfigDict`. The name of the file is the name of the variable: `app_database_url` fills
  the field `url` of a settings class with the prefix `APP_DATABASE_`. A variable of the same name
  in `.env` or in the environment wins over the file, so leave it out of `.env`.
- Set `path` and `target` of `develop.watch` to the folder of your source in the repository and in
  the image, and the `rebuild` path to the lock file of your package manager.
- Add a service of the stack, such as Redis, with its own healthcheck and a named volume when it
  keeps data, and add it to the `depends_on` of `app` with `condition: service_healthy`.
