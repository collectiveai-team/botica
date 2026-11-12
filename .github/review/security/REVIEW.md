---
role: security
applies:
  paths: []
  tiers: [low, medium, high]
stages: [local, ci]
blocking: P1
owners: ["@jedzill4"]
max-findings: 10
---

## Invariants

- No secret of the repository reaches code of a pull request from a fork. The secrets are read by
  `abide.yml` (`TYPESAFE_AI_API_KEY`, on `pull_request`, where a fork gets no secret),
  `botica-release.yml` (`MIRROR_DEPLOY_KEY`), `pins.yml` (`PINS_TOKEN`) and
  `dependabot-automerge-label.yml` (`DEPENDABOT_APP_PRIVATE_KEY`, `DEPENDABOT_METADATA_TOKEN`). A
  new secret, or a known one in a new workflow or on a new trigger, is checked against this list.
- A workflow on `pull_request_target` or `workflow_run` reads the pull request as data and never
  checks out or runs its code. Today these are `dependabot-automerge-label.yml` and
  `dependabot-automerge-run.yml`; `review.yml` follows the limits of ADR 0017 decision 9.
- The value of `BOTICA_HISTORY_KEY` never reaches an extension rule, an output, a log or the
  history: `botica-ext` and `botica ext sync` remove it from the environment of the process of an
  extension (`crates/botica-ext/src/lib.rs`, `crates/botica-cli/src/ext_cmd.rs`), and a fingerprint
  is a keyed hash of it (ADR 0019 decision 5). The other programs that botica starts (`git`, `uv`,
  `npx`, a package manager) inherit the environment, the key with it.
- botica starts a program only from the configuration of the project or the catalog: an extension
  rule, `uv`, `npx`, `git`. A file under lint, a finding or the text of a diff never becomes a
  program or an argument of one. `botica server` runs extension rules only with `--extensions` or
  the option `extensions` of `initialize` (`crates/botica-cli/src/server_cmd.rs`).
- botica runs `git` with `GIT_TERMINAL_PROMPT=0`, so that no command waits for a password, and
  passes a path after `--` or as `<rev>:<path>` (`crates/botica-cli/src/lint_cmd/git.rs`,
  `crates/botica-cli/src/refs_remote.rs`). A revision is not always guarded: `merge_base` and
  `commits` of `lint_cmd/git.rs` pass the value of `--diff` and `--commits` with no `--` or
  `--end-of-options`, so a value that starts with `-` reaches git as an option. Those flags come
  from the person or the workflow that runs botica, never from a file under lint.
- A path that botica writes or records stays inside the project: a path of a finding that leaves
  the root, as written or through a symbolic link, loses its path in the history
  (`crates/botica-cli/src/lint_cmd/inside.rs`), and `botica review export` refuses to write through
  a symbolic link (`crates/botica-cli/src/review_cmd/target.rs`).

## Check always

- A change to a workflow under `catalog/recipes/files/github*/` or `catalog/recipes/files/gitlab*/`
  is a change to the CI of every repository that installs the recipe: check its trigger, its
  `permissions` and the secrets it reads as if it ran here.
- A new `uses:`, `uvx` or `npx` pins a version the way the files around it do. Actions are pinned
  to a tag (`.github/zizmor.yml` sets `ref-pin`), and a tag pin is not a finding.
- A new place where botica starts a process (`Command::new`) builds its arguments from the
  configuration or the catalog only, removes `BOTICA_HISTORY_KEY` from the environment when the
  program is code of the project, and does not run through a shell.
- A new call to `git` that takes a revision from a value that a file, an artifact or a pull request
  can set puts `--end-of-options` before it, and a new caller of `merge_base` or `commits` of
  `lint_cmd/git.rs` takes the revision from a flag only.
- A new reader of a file that comes from outside the checkout (an artifact, a remote reference, a
  zip) bounds what it reads and keeps each path inside its folder.
- A change to a file that an agent loads as instructions or configuration (`AGENTS.md`,
  `.claude/`, `opencode.jsonc`, `catalog/recipes/files/claude-settings.json`,
  `catalog/recipes/files/opencode.jsonc`, a `SKILL.md`) grants no new permission to run commands
  and exposes no secret.
- A skill under `skills/review/` passes no text of the issue, the pull request or the diff to a
  shell command, and puts no command that the issue describes in `how_to_verify`.

## Do not report

- What zizmor reports on the workflows of `.github/` and of the recipes.
- A secret that matches a pattern of betterleaks, which the hook reports on each commit.
- A vulnerability of a dependency of `uv.lock`, which `osv-scanner` reports. `dependency-review`
  runs with `continue-on-error` and blocks nothing, so a new dependency is still reviewed here.
- A pin of an action to a tag instead of a commit.
- An argument of `Command::new` in a test under `crates/*/tests/`, which runs on the machine of the
  author with values that the test writes.

<!-- botica:covered -->
- The hook `betterleaks` of `prek.toml`, which the job `lint, type-check & test` of `tests.yml`
  also runs: secrets in the changes.
- The job `zizmor` of `zizmor.yml`: the workflows, actions and hooks of the repository.
- The job `osv-scanner (new in this PR)` of `osv-scanner.yml`: a vulnerability that the pull
  request adds to `uv.lock`.
- The test `recipes_workflows_lint` of `botica-catalog`, run by the job `fmt, clippy & test` of
  `ci.yml`: zizmor and actionlint on the workflows of the recipes.
- The rules `uv-cooldown`, `cargo-cooldown` and `dependabot-cooldown` of `[tool.botica]` in
  `pyproject.toml`.
<!-- /botica:covered -->

## Evidence

Each finding gives the path from the input to the use, with file and line at each step, and the
input that triggers it: a file, an argument, a branch name or an event of a pull request. It names
a secret by its name and the file that reads it, never by its value.
