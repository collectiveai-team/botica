## Directive

`prek.toml` has a hook with the id `docker-compose-config`, so that each Compose file of the
repository passes `docker compose config` before it is committed. A Compose file is a YAML file
whose name starts with `compose` or `docker-compose`, in any folder: `compose.yaml`,
`compose.override.yaml`, `docker-compose.prod.yml`, `.devcontainer/docker-compose.yml`.

## Why

`check-yaml` accepts a Compose file with a misspelled key (`imagee:`) or a value of the wrong type
(`ports: 8000`). Docker Compose reports it only at `docker compose up`, on the machine that deploys.
`docker compose config` checks the file against the schema of Compose without starting anything.

## Exceptions

A repository with no Compose file has nothing to check. `external-config` skips the rule unless a
file matches one of its `applies-when` globs, `**/compose*.y*ml` or `**/docker-compose*.y*ml`. The
hook has a `files` pattern, so in such a repository it does not run and does not need Docker.

## Tool

[`docker compose config`](https://docs.docker.com/reference/cli/docker/compose/config/) parses the
file, merges it, validates it against the schema of Compose and prints the result; `--quiet` prints
nothing and keeps the exit code. Two flags keep it from reading what is not committed:

- `--no-interpolate` leaves `${VARIABLE}` as it is written, so a variable that only the `.env` of a
  developer sets is not an error.
- `--no-env-resolution` does not read the files of `env_file`, so a missing `.env` is not an error.

It needs the Docker CLI with the Compose plugin on the `PATH`, and no running daemon.

## Configuration

The block `# MARK: docker-compose-config` of the template of `prek.toml` of the recipe `base`:

```toml
[[repos]]
repo = "local"

[[repos.hooks]]
id = "docker-compose-config"
name = "docker compose config"
entry = "sh -c 'status=0; for file do docker compose --file \"$file\" config --quiet --no-interpolate --no-env-resolution || status=1; done; exit $status' --"
language = "system"
files = '(^|/)(docker-)?compose[^/]*\.ya?ml$'
types = ["yaml"]
priority = 2
```

`docker compose config` takes the name of a service as its argument, not a file, so the entry runs
it once per file with `--file`. `external-config` checks that a hook with the id
`docker-compose-config` is there.

## Do

```yaml
services:
  api:
    image: "registry.example.com/api:${API_TAG}"
    env_file: .env
    ports:
      - "8000:8000"
```

## Don't

```yaml
services:
  api:
    imagee: registry.example.com/api
    ports: 8000
```

## Suppression

A file that is not a Compose file but has such a name goes in the `exclude` of the hook. The rule
itself, which only checks that the hook is there, is turned off in the configuration of botica.
