## Directive

Each manifest that declares dependencies has the lockfile of one package manager, next to it or in
a folder above it (the root of its workspace), and the lockfile is committed. The rule runs once
for the whole project, on every `botica lint`, and reports at the manifest. An error by default.

| Manifest | Declares dependencies when | Lockfiles |
|---|---|---|
| `pyproject.toml` | it has a `[project]` table | `uv.lock`, `poetry.lock`, `pdm.lock`, `Pipfile.lock` |
| `package.json` | `dependencies`, `devDependencies` or `optionalDependencies` is not empty | `pnpm-lock.yaml`, `package-lock.json`, `npm-shrinkwrap.json`, `yarn.lock`, `bun.lock`, `bun.lockb` |
| `go.mod` | it has a `require` directive | `go.sum` |
| `Cargo.toml` | it has a `[package]` or `[workspace]` table | `Cargo.lock` |

It reports a manifest when:

- no folder from the manifest up to the root of the project holds a lockfile of its ecosystem;
- its folder holds the lockfiles of two package managers, such as `package-lock.json` and
  `pnpm-lock.yaml`;
- the `packageManager` of a `package.json` names a manager (`pnpm@10.18.0`) and the lockfile found
  is that of another.

A lockfile that `.gitignore` leaves out counts as missing, because the walk of botica honours it.

## Why

Without a lockfile each install resolves the versions again, so CI and each machine can run
different code, and `osv-scanner` has no resolved set to check. Two lockfiles in one folder mean
two managers installed the project, often an agent that ran `npm install` in a repository of pnpm,
and the two sets of versions drift apart. A library commits its lockfile too: uv says that
`uv.lock` "should be checked into version control", and Cargo suggests committing `Cargo.lock` "as
a starting point".

## Do

```text
pyproject.toml
uv.lock
web/package.json        # "packageManager": "pnpm@10.18.0"
web/pnpm-lock.yaml
```

## Don't

```text
pyproject.toml          # no uv.lock, or uv.lock in .gitignore
web/package.json
web/package-lock.json
web/pnpm-lock.yaml
```

## Suppression

The rule is of project scope, and only the configuration of botica silences it: the manifest in
`extend-ignores` of `[rules.lockfile-committed]`, or `ignore = ["lockfile-committed"]`. A comment
that names it is `invalid-suppression`.
