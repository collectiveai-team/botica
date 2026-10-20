---
id: CES-119
title: Dependency vulnerability scanning
---

## Directive

Every push to `main` and every pull request is scanned for known vulnerabilities in dependencies
with [`osv-scanner scan source`](https://github.com/google/osv-scanner), against the lockfile of the
project: `uv.lock` for Python. A project of another ecosystem uses the same tool and the same
workflow with its own lockfile. The workflow is `osv-scanner.yml`, which the recipes `ci-python`,
`ci-go` and `ci-node` install; its comments hold the settings that it needs. A pull
request fails on the vulnerabilities that it introduces. `main`, the weekly schedule and a manual
run scan the whole lockfile.

## Why

`osv-scanner` reads the lockfiles of several ecosystems directly, `uv.lock` among them with no
`uv export` step, so a repository with a Python service and a Node or Go one needs one scanner.

A scan of the whole lockfile on a pull request fails every open pull request on the day a
vulnerability is disclosed against a dependency that is already on `main`, whatever the pull
request changes. The scan of a pull request compares the base with the head. The full scan runs on
`main`, and a vulnerability disclosed against a dependency that is already there fails it once.

## Suppression

An ignore for one vulnerability goes in an `osv-scanner.toml` file at the root of the repo (see the
[configuration of osv-scanner](https://google.github.io/osv-scanner/configuration/)). The whole
workflow is not silenced to unblock one accepted advisory.
