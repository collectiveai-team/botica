---
id: CES-114
title: Dockerfile and Compose lint
---

## Directive

Each Dockerfile of a repo, in any folder, passes [`hadolint`](https://github.com/hadolint/hadolint)
as a hook of prek. It reports unpinned base images, a missing `--no-install-recommends`, `ADD`
where `COPY` belongs, running as root, the shell errors that ShellCheck finds in `RUN`, and similar
problems. A finding of severity `warning` or `error` fails the hook; an `info` or `style` finding
is printed and does not.

Each Compose file of a repo, in any folder, passes
[`docker compose config`](https://docs.docker.com/reference/cli/docker/compose/config/) as a hook
of prek, which checks it against the schema of Compose.

## Why

Scanners such as `trivy` and `grype` work on the built image. `hadolint` checks the text of the
Dockerfile itself: it is deterministic, it runs as a prek hook, and it needs no build and no
Docker.

`check-yaml` accepts a Compose file with a misspelled key (`imagee:`) or a value of the wrong type
(`ports: 8000`), and Docker Compose reports it only at `docker compose up`.

## Scope

A Dockerfile is a file that prek tags `dockerfile`: `Dockerfile`, `Containerfile`,
`Dockerfile.<name>` and `<name>.Dockerfile`, in any folder, `.devcontainer/` included. The hook
keeps the `types = ["dockerfile"]` of hadolint and sets no `files`, so it runs on all of them and
does nothing in a repo without one. A file such as `Dockerfile.md` is Markdown and is skipped.

The hook is `hadolint` of `https://github.com/AleksaC/hadolint-py`, which installs the release
binary of hadolint that its `rev` names, checked against its SHA-256, in the environment of the
hook. The hook `hadolint-docker` of the hadolint repository runs the image
`ghcr.io/hadolint/hadolint` with no tag, so it takes `latest` whatever the `rev`, and it fails on a
machine without a running Docker daemon.

A Compose file is a YAML file whose name starts with `compose` or `docker-compose`, in any folder:
`compose.yaml`, `compose.override.yaml`, `docker-compose.prod.yml`,
`.devcontainer/docker-compose.yml`. The hook `docker-compose-config` is local to the repo: it runs
the Docker CLI of the `PATH`, with its Compose plugin, and no daemon. Its `files` pattern keeps it
from running, and from needing Docker, in a repo without a Compose file.

## Configuration

The hadolint hook passes `--failure-threshold=warning` and `--ignore=DL3008`. DL3008 asks for the
version of each package of `apt-get install`; Debian and Ubuntu drop the previous version of a
package from their archives after an update, so a pinned version breaks the next build
([hadolint#884](https://github.com/hadolint/hadolint/issues/884)). The base image is pinned
instead (DL3006, DL3007).

A flag of the hook wins over the same key of `.hadolint.yaml`: a repo that wants another
threshold changes the `args` of the hook. The `ignored` list of `.hadolint.yaml` adds to the one
of the hook.

The Compose hook runs `docker compose --file <file> config --quiet --no-interpolate
--no-env-resolution` once per file. `--no-interpolate` leaves `${VARIABLE}` as written and
`--no-env-resolution` does not read the files of `env_file`, so the `.env` that is not committed
is not needed. The check is the schema, not the values a deploy fills in.

## License

`hadolint` is GPL-3.0 and runs as an external command through prek, the boundary that every other
third-party tool of this stack has (ruff, betterleaks), so it puts no copyleft obligation on the
source of the repo.

## Migration (existing repos)

Replace the block `# MARK: hadolint-docker` of `prek.toml` with the block `# MARK: hadolint` of the
template of the recipe `base`, add the block `# MARK: docker-compose-config`, then run
`prek run hadolint --all-files` and `prek run docker-compose-config --all-files`. Fix the findings,
or ignore those of hadolint explicitly with `# hadolint ignore=DLxxxx`. The posture is the one of
`file-size-guard` (CES-71): flag, and do not exempt silently.

## Suppression

Inline, with `# hadolint ignore=DLxxxx` on the line above the instruction, or in the `ignored` list
of a `.hadolint.yaml` at the root of the repo, for the whole project. A YAML file whose name looks
like a Compose file and is not one goes in the `exclude` of the Compose hook. A hook is not
disabled globally to silence one rule.
