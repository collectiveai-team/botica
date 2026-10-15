---
id: CES-109
title: dependency hygiene (deptry)
---

**Enforced by:** prek hook

## Directive

`pyproject.toml`'s declared dependencies must match the actual import graph, enforced via
[`deptry`](https://github.com/fpgmaas/deptry): no declared-but-unused deps (`DEP002`), no
imported-but-undeclared deps (`DEP001`), no dev deps imported in prod code or vice versa
(`DEP004`).

## Why

`pip-audit`/`osv-scanner` check the resolved dependency set for CVEs but never check whether the
*declared* set matches what's actually *imported*. An agent that tries an approach mid-session and
reverts frequently leaves dependencies declared-but-unused, or imports something that rode in
transitively without ever being declared. This is a distinct hygiene signal ruff doesn't provide.

## Scope and false positives

Dynamic imports, plugin entry points, and `TYPE_CHECKING`-only imports are the known noise
sources — configure `[tool.deptry]` on first rollout rather than disabling the hook. This tool's
own `pyproject.toml` demonstrates the pattern:

```toml
[tool.deptry]
extend_exclude = ["scaffolding/templates", "skills"]
```

`scaffolding/templates` and `skills` are bundled/vendored payload shipped verbatim into target
repos (the same rationale as this repo's ruff `extend-exclude` and pyrefly `project-excludes`).

## Migration (existing repos)

Run `uvx deptry .` once. Either fix the findings or seed a `[tool.deptry]` `extend_exclude` /
`per_rule_ignores` for the pre-existing/intentional cases — same "flag, don't silently exempt"
posture as `file-size-guard` (CES-71).

## Suppression

Per-package: `--per-rule-ignores DEP001=somepkg`. Per-path: `extend_exclude` in
`[tool.deptry]`. Never disable the hook globally to silence a single path.
