# botica lint reads a diff

**Status:** accepted

`botica lint` reads the files as they are. Two rules of the plan need what changed: CES-66 reports
a `# pragma: no cover` or an exclusion of coverage that a change adds and a `fail_under` that a
change lowers, and the cognitive complexity of step 6 reports the functions that a change touches.
A repository with old findings cannot turn a rule on until every old finding is fixed, because
`botica lint` reports them all on each run. ADR 0003 left `--diff <base>` for later.

## Decision

1. **Three inputs.** `botica lint --diff <rev>` compares the working tree with the merge base of
   `<rev>` and `HEAD`: in CI, `--diff origin/main` gives the changes of the pull request, and on a
   machine it also gives the changes not yet committed. `botica lint --staged` compares the index
   with `HEAD` and reads the files as they are staged, for the pre-commit hook.
   `botica lint --diff-file <patch>` reads a unified diff, as `git diff` writes it, for a runner
   with no history. The three options exclude each other.
2. **The files of the diff.** With a diff and no paths, `botica lint` reads the files that the diff
   adds or changes; a deleted file is not read. With paths, it reads the files of the diff under
   those paths. `files` and `ignores` of the configuration apply as before.
3. **Every finding is filtered.** With a diff, a finding with a range is kept when the range touches
   a line that the diff adds or changes. A finding of a whole file is kept when the diff adds or
   changes the file. A finding with no path, of a rule of scope `project`, is kept. The filter
   applies to the findings of every kind of rule, extensions and `--judged` included, and runs
   after the configuration and the `# botica: ignore[<slug>]` comments.
4. **The base of a file.** A native rule can read the text of a file before the change, from the
   `Document` it is given: `None` without a diff and for a file that the diff adds. `--diff <rev>`
   takes the base from the merge base and `--staged` from `HEAD`. `--diff-file` builds it by
   applying the hunks of the patch in reverse to the file of the working tree; when they do not
   apply, the patch does not describe the tree, and `botica lint` exits 2.
5. **The rules of CES-66.** `no-coverage-pragma` (BTTEST004, ast-grep, Python) reports a comment
   `# pragma: no cover`. `no-coverage-exclusion` (BTTEST005, native) reports an entry of `omit`,
   `exclude_lines` or `exclude_also` of coverage in `pyproject.toml`, `.coveragerc` and
   `setup.cfg`, and a `fail_under` lower than the one of the base. Without a diff, both report what
   is there, and `fail_under` is not reported, since there is no earlier value. Both are preview
   and off by default.
6. **The judgment rule of CES-66 is split.** `test-coverage-gap` keeps the test that runs lines
   without asserting a behaviour, with no `covered-by`. A new judgment rule,
   `coverage-exclusion-reachable`, takes the exclusion of coverage that hides code a test could
   reach, and its `covered-by` names the two rules of decision 5. When one of them is active, the
   new rule leaves the block of `AGENTS.md` and the rubric, and `test-coverage-gap` stays.

## Considered options

- **Only the rules that ask for the diff read it.** Rejected: the other rules would keep reporting
  every old finding, and a repository with old findings still could not turn a rule on.
- **Only `--diff-file`.** Rejected: a hook and a CI job would each have to write the patch first,
  and the base of a file would always come from the patch.
- **A baseline file of known findings.** Left out of this ADR: a baseline lists the findings by
  their range, which moves when the code above them changes. Step 6 decides whether it needs one.

## Consequences

- The `botica` hook of the mirror and of `prek.toml` does not change: prek passes the staged files,
  and `--staged` is added to a hook only when a project wants the filter of decision 3.
- `botica server` reads no diff; an editor shows every finding of the buffer.
- A change that breaks a rule on a line it does not touch is not reported with a diff, such as an
  import that a removed line made unused. Without a diff the finding is reported.
- A run with a diff costs one `git diff`, and one `git show` per file whose base a native rule
  reads.
