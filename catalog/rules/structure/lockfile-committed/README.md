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

A root module of Terraform or OpenTofu has its `.terraform.lock.hcl` in its own folder, the one
where `terraform init` runs, and no folder above it stands in for it. A folder of `.tf` or `.tofu`
files is a root module when one of its files has a `provider`, a `backend` or a `cloud` block,
which only a root module carries, and no `module` block of the project calls it with a local
`source` such as `"../modules/network"`. It needs the lockfile when it has a `provider` or a
`required_providers` block. The rule reports at the first file of the folder with the `provider`,
`backend` or `cloud` block. A reusable module, which declares `required_providers` and no
`provider` block, needs no lockfile: the lockfile of the root module that calls it pins its
providers.

A lockfile that `.gitignore` leaves out counts as missing, because the walk of botica honours it.

## Why

Without a lockfile each install resolves the versions again, so CI and each machine can run
different code, and `osv-scanner` has no resolved set to check. Two lockfiles in one folder mean
two managers installed the project, such as `npm install` run in a repository of pnpm,
and the two sets of versions drift apart. A library commits its lockfile too: uv says that
`uv.lock` "should be checked into version control", and Cargo suggests committing `Cargo.lock` "as
a starting point". Terraform records in `.terraform.lock.hcl` the version and the checksums of each
provider, and its documentation says to include the file in version control; it belongs to the
configuration as a whole, so it lives in the folder of the root module
(https://developer.hashicorp.com/terraform/language/files/dependency-lock).

## Do

```text
pyproject.toml
uv.lock
web/package.json        # "packageManager": "pnpm@10.18.0"
web/pnpm-lock.yaml
infra/live/main.tf      # provider "aws" {} and module "network" { source = "../modules/network" }
infra/live/.terraform.lock.hcl
infra/modules/network/main.tf   # required_providers only: the lockfile of infra/live pins it
```

## Don't

```text
pyproject.toml          # no uv.lock, or uv.lock in .gitignore
web/package.json
web/package-lock.json
web/pnpm-lock.yaml
infra/live/main.tf      # provider "aws" {}, and .terraform.lock.hcl in .gitignore
```

## Suppression

The rule is of project scope, and only the configuration of botica silences it: the manifest in
`extend-ignores` of `[rules.lockfile-committed]`, or `ignore = ["lockfile-committed"]`. A comment
that names it is `invalid-suppression`. A repository of a reusable module whose `examples/` are root
modules that it does not lock puts `"examples/**"` in `extend-ignores`.
