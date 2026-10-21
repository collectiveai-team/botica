---
id: CES-119
title: Dependency vulnerability scanning
---

## Directive

Every push to `main` and every pull request is scanned for known vulnerabilities in dependencies
with [`osv-scanner scan source`](https://github.com/google/osv-scanner), against the lockfile of the
project: `uv.lock` for Python. A project of another ecosystem uses the same tool and the same
workflow with its own lockfile. The workflow is `osv-scanner.yml`. A pull request fails on the
vulnerabilities that it introduces. `main`, the weekly schedule and a manual run scan the whole
lockfile.

## Why

`pip-audit` understands Python only. `osv-scanner` (Google, Apache-2.0) uses the same OSV.dev
vulnerability database and parses the lockfiles of about 20 ecosystems directly, `uv.lock` among
them with no `uv export` step, and also `poetry.lock`, `Pipfile.lock`, `package-lock.json`,
`Cargo.lock` and `go.mod`. A repo that also has a Next.js frontend or a Go service needs no second
scanner.

A scan of the whole lockfile on a pull request fails every open pull request on the day a
vulnerability is disclosed against a dependency that is already on `main`, whatever the pull
request changes. The scan of a pull request compares the base with the head. The full scan runs on
`main`, and a vulnerability disclosed against a dependency that is already there fails it once.

## Configuration

The workflow has two jobs. Each calls a reusable workflow of `google/osv-scanner-action` with
`scan-args: --lockfile=uv.lock`. `uv.lock` is recognized by its file name, so the argument needs no
prefix that forces a parser.

- `osv-scanner-reusable.yml@v2.7.0` runs on every event except `pull_request` (`main`, the
  schedule and `workflow_dispatch`) and scans the whole lockfile.
- `osv-scanner-reusable-pr.yml@v2.7.0` runs on `pull_request`, scans the base and the head, and
  fails on the vulnerabilities that only the head has.

Two settings of the caller fail at startup, with no jobs, and not as a scan error:

- The reference is an exact tag. The action publishes no floating `v2` tag or branch, so `@v2`
  cannot be resolved.
- Each calling job has `security-events: write`, `actions: read` and `contents: read`. A called
  workflow cannot hold more permissions than its caller, and the reusable job needs them to upload
  SARIF.

A private repository without GitHub Code Security cannot upload SARIF. It adds `upload-sarif: false`
to both jobs. `fail-on-vuln` is `true` by default: the check fails on a finding and the job log
lists it.

A branch protection rule that requires the scan of a pull request names the job
`osv-scanner (new in this PR)`. The `osv-scanner` job is skipped on a pull request, and GitHub
counts a skipped job as passing.

## Migration (existing repos)

A repo that has `pip-audit.yml` deletes it when it adopts `osv-scanner.yml`. Both report the same
class of finding from the same database, and running both only adds noise.

## Suppression

An ignore for one vulnerability goes in an `osv-scanner.toml` file at the root of the repo (see the
[configuration of osv-scanner](https://google.github.io/osv-scanner/configuration/)). The whole
workflow is not silenced to unblock one accepted advisory.
