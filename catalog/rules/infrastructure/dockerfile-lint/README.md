## Directive

`prek.toml` has a hook with the id `hadolint`, so that each Dockerfile of the repository passes
hadolint before it is committed. A Dockerfile is a file that prek tags `dockerfile`:
`Dockerfile`, `Containerfile`, `Dockerfile.<name>` and `<name>.Dockerfile`, in any folder. A
finding of severity `warning` or `error` fails the hook.

## Why

A Dockerfile that installs packages with `apt` instead of `apt-get`, copies with `ADD`, pipes in
`RUN` without `pipefail` or runs as root still builds, and the image still runs. hadolint reads the
text of the Dockerfile and reports these before a build.

## Exceptions

A repository with no Dockerfile has nothing to lint. `external-config` skips the rule unless a
file matches one of its `applies-when` globs: `**/Dockerfile`, `**/Dockerfile.*`,
`**/*.Dockerfile` or `**/Containerfile`.

## Tool

[hadolint](https://github.com/hadolint/hadolint) parses the Dockerfile and runs ShellCheck on each
`RUN`. The hook `hadolint` of `https://github.com/AleksaC/hadolint-py` installs the release binary
of hadolint that its `rev` names, checked against its SHA-256, so it needs neither Docker nor
hadolint on the `PATH`. The hook `hadolint-docker` of the hadolint repository does not count: it
runs the image `ghcr.io/hadolint/hadolint` with the tag `latest`, whatever its `rev`, and fails on
a machine with no Docker daemon.

## Configuration

The block `# MARK: hadolint` of the template of `prek.toml` of the recipe `base`:

```toml
[[repos]]
repo = "https://github.com/AleksaC/hadolint-py"
rev = "v2.15.1"
hooks = [{ id = "hadolint", args = ["--failure-threshold=warning", "--ignore=DL3008"], priority = 2 }]
```

`external-config` checks that a hook with the id `hadolint` is there, from any repository: the
hook `hadolint` of the hadolint repository, which runs the `hadolint` of the `PATH`, counts too.

## Do

```dockerfile
FROM python:3.13-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl \
    && rm -rf /var/lib/apt/lists/*
COPY app.py /app/
USER 10001
```

## Don't

```dockerfile
FROM python
RUN apt update && apt install -y curl
ADD app.py /app/
```

## Suppression

`# hadolint ignore=DL3020` on the line above the instruction, or the code in the `ignored` list of
`.hadolint.yaml` at the root of the repository. The rule itself, which only checks that the hook
is there, is turned off in the configuration of botica.
