# botica lint reads commit messages

**Status:** accepted

Amended 2026-10-23: ADR 0013 replaces the second item of decision 6 on the hooks of botica. The
four hooks of botica in `prek.toml` run the checkout too, and `prek.toml` no longer names the
mirror. The `--select` of the hooks of commit-msg and of the two workflows stays, because they copy
the templates, where each one checks one rule; the release of 0.13.0 is no longer its reason.

CES-75 (Conventional Commits) and CES-91 (no AI co-authorship) are checked by Python regular
expressions written four times: in the `commit-msg` hooks of `prek.toml` and of
`catalog/recipes/files/prek-base.toml`, and in the workflows `conventional-commits.yml` and
`commit-policy.yml` with their templates. No rule of botica implements either standard, so
`botica rules` does not list them, their configuration is the text of the regular expression, and
the copies can drift.

## Decision

1. **Three inputs.** `botica lint --commit-msg <file>` reads one message from a file, as git gives
   it to a `commit-msg` hook. `botica lint --commits <range>` reads the message of each commit of
   `git log <range>`. `botica lint --pr-title=<title>` reads the title of a pull request; a
   workflow passes it from an environment variable (`--pr-title="$PR_TITLE"`), never written into
   the script. The three exclude each other and exclude paths, `--diff`, `--staged`,
   `--diff-file` and `--judged`: a run reads messages or files, not both.
2. **Rules of scope `commit`.** A rule with `scope = "commit"` in its `rule.toml` reads a message
   and never a file; the other rules never read a message. Such a rule is native, and a run with
   one of the three inputs runs only these rules.
3. **Where a finding is.** A finding of `--commits` has the path `commit <short sha>` and the line
   of the message; one of `--commit-msg` has the path of the file and the line; one of
   `--pr-title` has the path `pull request title` and line 1. A message has no comments that botica
   reads, so only the configuration silences a finding: the rule in `ignore`, or its `enabled`.
4. **`conventional-commits` (BTPROC003).** The subject, the first line of the message, is
   `<type>(<scope>)!: <description>`, with the scope and the `!` optional, a lowercase scope, and a
   type of the option `types` (by default the eleven of CES-75). A subject that starts with an
   entry of the option `exempt` (by default `Merge `, `Revert ` and `fixup! `) is not checked.
5. **`no-ai-coauthorship` (BTPROC004).** A line of the message is reported when it is a
   `Co-authored-by:`, `Generated with` or `by`, `AI-generated-by:` or `Assisted-by:` line that
   names a tool of the option `tools` (by default the list of CES-91), in any case.
6. **The checks move to botica now.** Both rules start in preview and off by default, and the
   recipe `base` selects them.
   - The templates: the `commit-msg` hook of `prek-base.toml` runs `botica lint --commit-msg`,
     `conventional-commits.yml` runs `botica lint --pr-title` and `commit-policy.yml` runs
     `botica lint --commits`, each with `uvx botica@X.Y.Z`, which `recipe install` sets to its
     release.
   - This repository runs the botica of its checkout: the hook with `cargo run`, the workflows
     with the binary they build, as `abide.yml` does. Each names its rules with `--select`,
     because the hooks of the mirror run 0.13.0, which fails on a rule it does not know in the
     `select` of `pyproject.toml`. The `select` lists them after the next release.

## Considered options

- **Keep the regular expressions in the hooks and the workflows.** Rejected: four copies, and
  configuration that is a regular expression.
- **A run that reads files and messages at once.** Rejected: a hook of commit-msg and a job of CI
  each need one of them, and a run of one kind is simpler to read.
- **A message as a file in a temporary directory, linted by the rules of files.** Rejected: every
  rule of files would need to leave it alone, and its path would mean nothing.

## Consequences

- `botica rules` lists the rules of CES-75 and CES-91, and `botica explain` gives their text.
- The jobs check what they checked before: `commit-policy.yml` runs `--select no-ai-coauthorship`
  over the commits, and `conventional-commits.yml` runs `--select conventional-commits` over the
  title, since a squash merge takes the title as the subject.
- A hook of commit-msg built with `cargo run` compiles botica on the first commit after a change to
  its code.
