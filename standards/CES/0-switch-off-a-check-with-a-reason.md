---
id: CES-0
title: A change does not switch off a check without saying why
---

## Directive

A change that switches off a check gives the reason next to it, in the place where the tool shows
it.

- A skipped or xfailed test carries its reason: `reason="..."` on `@pytest.mark.skip`,
  `@pytest.mark.skipif` and `@pytest.mark.xfail`, and the text of `pytest.skip("...")` and
  `pytest.xfail("...")`.
- A step of a workflow with `continue-on-error: true` says, in a comment on that line, why its
  failure does not fail the job.
- A command of a gate followed by `|| true` says, in a comment on that line, why its failure does
  not fail the gate.

The reason names what the check waits for: the bug, the platform, the missing dependency or the
issue that tracks it.

## Why

A check that is switched off passes with no output. The test report lists a skipped test as
`unconditional skip` or `Skipped`, and a job with `continue-on-error: true` or a command with
`|| true` stays green when the check fails. Without a reason, a reader cannot tell whether the
check is still off for the cause that turned it off, so nobody turns it back on.

The evidence is in the change that adds the skip, not in the state of the files: a rule that reads
every file reports each old skip of the repository, and a repository with many of them never turns
it on. The rules of this standard report what a change adds: run with `botica lint --diff
origin/main`, they keep the findings on the lines that the pull request adds, so a repository with
old skips can turn them on and fix the old ones when it has time.

pytest asks for a reason only of a `skipif` or an `xfail` whose condition is a boolean, and only
when the test runs. ruff 0.17.1 has no rule for a skip or an xfail without a reason; PT016 asks
for the message of `pytest.fail()`.

## Examples

```python
@pytest.mark.skipif(sys.platform == "win32", reason="no fork on Windows")
def test_the_worker_restarts(): ...


@pytest.mark.xfail(reason="#87: totals round half down", strict=True)
def test_the_total_rounds_half_up(): ...
```

```python
@pytest.mark.skip
def test_the_worker_restarts(): ...


@pytest.mark.xfail
def test_the_total_rounds_half_up(): ...
```

The first tests say why they do not run or why they fail, and pytest prints the reason in its
report. The second ones are reported as `unconditional skip` and `XFAIL` with no reason.
