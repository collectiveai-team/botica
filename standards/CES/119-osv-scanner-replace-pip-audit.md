---
id: CES-119
title: Dependency vulnerability scanning
---

## Directive

Every push to `main` and every pull request is scanned for known vulnerabilities in dependencies
with [`osv-scanner scan source`](https://github.com/google/osv-scanner), against the lockfile of the
project: `uv.lock` for Python. A project of another ecosystem uses the same tool and the same
workflow with its own lockfile. The workflow is `osv-scanner.yml`.

## Why

`pip-audit` understands Python only. `osv-scanner` (Google, Apache-2.0) uses the same OSV.dev
vulnerability database and parses the lockfiles of about 20 ecosystems directly, `uv.lock` among
them with no `uv export` step, and also `poetry.lock`, `Pipfile.lock`, `package-lock.json`,
`Cargo.lock` and `go.mod`. A repo that also has a Next.js frontend or a Go service needs no second
scanner.

## Configuration

The workflow calls the reusable workflow
`google/osv-scanner-action/.github/workflows/osv-scanner-reusable.yml@v2.5.0` with
`scan-args: --lockfile=uv.lock`. `uv.lock` is recognized by its file name, so the argument needs no
prefix that forces a parser.

Two settings of the caller fail at startup, with no jobs, and not as a scan error:

- The reference is an exact tag. The action publishes no floating `v2` tag or branch, so `@v2`
  cannot be resolved.
- The calling job has `security-events: write`, `actions: read` and `contents: read`. A called
  workflow cannot hold more permissions than its caller, and the reusable job needs them to upload
  SARIF.

## Migration (existing repos)

A repo that has `pip-audit.yml` deletes it when it adopts `osv-scanner.yml`. Both report the same
class of finding from the same database, and running both only adds noise.

## Suppression

An ignore for one vulnerability goes in an `osv-scanner.toml` file at the root of the repo (see the
[configuration of osv-scanner](https://google.github.io/osv-scanner/configuration/)). The whole
workflow is not silenced to unblock one accepted advisory.
