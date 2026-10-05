# CES-32 · keep non-code out of the package

**Code:** `CES-32` &nbsp;·&nbsp; **Slug:** `repo-shape` &nbsp;·&nbsp; **Enforced by:** prek hook
(path glob, parametrized) &nbsp;·&nbsp; **Tracker:**
[#32](https://github.com/collectiveai-team/botica/issues/32)

## Directive

The import package holds **code**, not artifacts. The `repo-shape` prek hook fails on Jupyter
notebooks (`*.ipynb`) and on `resources/`, `reports/`, and `data/` directories found *inside* the
import package. Keep those at the repo root (or a sibling), not under the importable package.

## Why

- Notebooks, datasets, and generated reports are not importable code. Bundling them into the
  package bloats the wheel, leaks large/binary files into installs, and blurs the line between
  "the library" and "the workspace around it".
- A clean package boundary means `pip install` ships only what's importable, and the repo's
  exploratory/output material lives where humans (not the import system) expect it.

## The `{{ import_package }}` placeholder

This hook is parametrized. The shipped `prek.toml` contains a copier-style
`{{ import_package }}` placeholder in the hook's `files` pattern; **resolve it at install time**
to your top-level package directory (e.g. `myapp`). The install guide instructs this fill. Until
it is resolved the pattern matches nothing, so the hook is inert — fill it to turn the guard on.

## Suggested layout

```text
repo/
  myapp/            # import package — code only
  data/             # datasets       — outside the package
  reports/          # generated output
  resources/        # static assets
  notebooks/        # *.ipynb exploration
```

## Suppression

`repo-shape` is a prek hook, so there is no per-line ast-grep ignore. To allow a specific path,
narrow the hook's `files`/`exclude` pattern in `prek.toml` rather than removing the guard.
