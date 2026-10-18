<p align="center">
  <img src="assets/logo.png" alt="botica — the house engineering standards, their linter and the recipes that install them" style="width:600px; max-width:100%; height:auto;" />
</p>

<p align="center">
  <em>The house engineering standards, the linter that measures them, and the recipes that set up a repository.</em>
</p>

<p align="center">
  <a href="https://github.com/collectiveai-team/botica/releases"><img alt="Release" src="https://img.shields.io/github/v/release/collectiveai-team/botica?logo=github" /></a>
  <a href="https://pypi.org/project/botica/"><img alt="PyPI" src="https://img.shields.io/pypi/v/botica?logo=pypi&logoColor=white" /></a>
  <a href="https://opencode.ai/"><img src="https://img.shields.io/badge/Agents-opencode%20%C2%B7%20claude--code%20%C2%B7%20codex-1d1d1d?logo=anthropic&logoColor=white" alt="opencode · claude-code · codex"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT"></a>
</p>

---

**Source Code**: [https://github.com/collectiveai-team/botica](https://github.com/collectiveai-team/botica)

---

This repository has:

- `standards/` — the text of each house standard, `CES-<number>` (see
  [`docs/engineering-standards.md`](docs/engineering-standards.md)).
- `lint/` — `botica`, published on PyPI: `botica lint` measures the rules of the catalog, and
  `botica recipe` installs the recipes. Its manual is [`lint/README.md`](lint/README.md).
- `skills/` — the agent skills of the house.
- `website/` — the site of the standards and the rules.

## Set up a repository

Run from the root of the repository, with [`uv`](https://docs.astral.sh/uv/) on the `PATH` (and
`npx` for the recipes that install skills):

```bash
uvx botica recipe list                                     # the recipes and their steps
uvx botica recipe install base base-python agent-claude-code ci-python repo-public
uvx botica recipe check base base-python agent-claude-code ci-python repo-public
```

`install` writes what is missing and leaves an existing file as it is: it reports the file as
`pending` and exits with 1. Commit `skills-lock.json` afterwards (CES-107); `.agents/skills/` is
gitignored and `npx skills experimental_install` restores it from the manifest.

| Recipe | What it installs |
| --- | --- |
| `base` | Any repository: the rules about any file, the skills, `.gitignore`, the prek hooks and the sections of `AGENTS.md` |
| `base-python` | A Python project: `base`, the rules about Python code, `pyproject.toml`, the Python hooks of prek and the dev tools |
| `agent-opencode` | OpenCode: the skills for opencode and its `opencode.jsonc` |
| `agent-claude-code` | Claude Code: `.claude/settings.json` and the links to `AGENTS.md` and `.agents/skills` |
| `agent-codex` | Codex: the skills for codex |
| `ci` | GitHub Actions: Conventional Commits, the commit policy and zizmor |
| `ci-python`, `ci-node`, `ci-go` | GitHub Actions for one ecosystem: dependabot, osv-scanner, and the tests of Python or Go |
| `ci-dependency-review` | The dependency review of the pull requests (free on a public repository) |
| `ci-docker` | Build and push an image to GHCR (needs a `Dockerfile`) |
| `ci-publish` | Release and publish to PyPI |
| `ci-automerge-public`, `ci-automerge-private` | Auto-merge of the Dependabot patch updates, with or without branch protection |
| `repo-public`, `repo-private` | `ci` and the workflows that fit the visibility of the repository |
| `secrets-varlock` | The skill of varlock; `varlock init --agent` writes the draft of `.env.schema` |

A project adds its own recipes in `recipes/` (ADR
[0006](docs/adr/0006-external-rules-and-recipes.md)).

## The skills of this repository

The recipe `base` installs them with `npx skills add collectiveai-team/botica`, beside the
skills of Matt Pocock that its `[[skills]]` step pins to a tag.

- `skills/productivity/journalist` — local daily session journals under `.journals/`.
- `skills/productivity/handoff` — compact the current session into a handoff for another agent.
- `skills/productivity/ask-user` — route a situation to the smallest suitable installed workflow.
- `skills/quality/test-smell-review` — review tests for false-green smells (tests that pass
  without protecting anything).
- `skills/engineering/engineering-rules` — the house engineering rules that no tool measures,
  under `references/` and indexed by `references/_sections.md`.
- `skills/engineering/engineering-pr-review` — the house PR gate: the shared rules, the
  deep-module check and the large-file check, run before a PR is opened.
- `skills/engineering/engineering-refactor` — move an existing repository toward the house
  standard (toolchain, layout, tests, architecture) and keep its local contracts.

`npx skills add` and `skills update` overwrite the installed copies in place, so local edits go
in this repository's `skills/`, not in `.agents/skills`.
