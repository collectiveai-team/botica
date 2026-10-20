---
id: CES-109
title: Dependency hygiene
---

## Directive

The dependencies that `pyproject.toml` declares match the import graph, checked by
[`deptry`](https://github.com/fpgmaas/deptry): no declared dependency that is unused (`DEP002`), no
imported dependency that is undeclared (`DEP001`), and no dev dependency imported in production
code or the reverse (`DEP004`).

In every ecosystem, each manifest that declares dependencies (`pyproject.toml`, `package.json`,
`go.mod`, `Cargo.toml`) has the lockfile of one package manager, next to it or at the root of its
workspace, and the lockfile is committed. A folder holds the lockfile of one manager only, and the
`packageManager` of a `package.json`, when it names one, names the manager of that lockfile. This
holds for libraries too: uv says that `uv.lock` "should be checked into version control", and
Cargo suggests committing `Cargo.lock` "as a starting point".

## Why

`osv-scanner` checks the resolved dependency set for CVEs. It does not check whether the
declared set matches what is imported. An agent that tries an approach mid-session and reverts
often leaves dependencies declared and unused, or imports something that came in transitively and
was never declared. Ruff gives no signal about this.

A lockfile fixes the versions that CI and each machine install, and it is the set that
`osv-scanner` checks. An agent that runs `npm install` in a repository of pnpm leaves a second
lockfile, and the two sets of versions drift apart.

## Scope and false positives

Dynamic imports, plugin entry points and imports under `TYPE_CHECKING` are the known sources of
noise. They are configured in `[tool.deptry]` on the first rollout and the hook stays on:

```toml
[tool.deptry]
extend_exclude = ["vendor"]
```

`vendor` stands for a folder of vendored code that has dependencies of its own.

## Migration (existing repos)

Run `uvx deptry .` once. Either fix the findings or add an `extend_exclude` or `per_rule_ignores`
entry to `[tool.deptry]` for the pre-existing or intentional cases. The posture is the one of
`file-size-guard` (CES-71): flag, and do not exempt silently.

## Suppression

Per package: `--per-rule-ignores DEP001=somepkg`. Per path: `extend_exclude` in `[tool.deptry]`. The
hook is not disabled globally to silence one path.
