## Directive

A pytest skip or xfail without a reason is reported: `@pytest.mark.skip`, `@pytest.mark.skipif`
and `@pytest.mark.xfail` with no `reason=`, also in `pytestmark = ...` and in
`pytest.param(..., marks=...)`, and `pytest.skip()` and `pytest.xfail()` with no text. The text of
`@pytest.mark.skip("...")`, `pytest.skip("...")` and `pytest.xfail("...")` can be their first
argument; the first argument of `skipif` and `xfail` is a condition, so their reason is
`reason=`. A reason that is an empty string is reported, and one that is a name or an f-string is
not. The names `mark`, `skip` and `xfail` imported with `from pytest import ...` are read too, and
`@unittest.skip` with no text. A warning by default.

The rule is meant to run on a diff. `botica lint --diff origin/main` keeps the findings on the
lines that the change adds, so a repository with old skips can turn it on. In CI, the base must be
in the clone: `actions/checkout` fetches one commit by default, and `botica lint --diff
origin/main` then exits 2 with `git merge-base origin/main HEAD failed`. Give the checkout
`fetch-depth: 0`, or fetch the base before the run.

## Why

pytest reports a skip with no reason as `unconditional skip` or `Skipped`, and an xfail with no
reason as `XFAIL` alone. A reader of the report cannot tell whether the cause still holds, so the
test stays off after its bug is fixed. pytest asks for a reason only of a `skipif` or an `xfail`
whose condition is a boolean, and only when the test runs. ruff 0.17.1 has no rule for it.

## Exceptions

The rule does not report `pytest.importorskip()`, whose report names the module that is missing.
It does not report `unittest.skipIf`, `unittest.skipUnless` and `self.skipTest`, which raise
`TypeError` without their reason, or `unittest.expectedFailure`, which takes none. It reads a mark
by its name: `xfail = pytest.mark.xfail` is reported even when each use of `xfail(...)` gives a
reason, and a module imported under another name (`import pytest as pt`) is not read.

## Do

```python
@pytest.mark.skipif(sys.platform == "win32", reason="no fork on Windows")
def test_the_worker_restarts(): ...


def test_the_export(tmp_path):
    if not (tmp_path / "data").exists():
        pytest.skip("the data set is not downloaded")
```

## Don't

```python
@pytest.mark.skip
def test_the_worker_restarts(): ...


@pytest.mark.xfail(raises=KeyError)
def test_the_export(): ...
```

## Suppression

One comment silences one finding, on the line of the mark or the call. There are two forms:
`# botica: ignore[<slug>]` names the rule by its slug, and `# noqa: <code> -- <reason>` names it
by its code, with the reason after `--`.

```python
req.applymarker(pytest.mark.xfail)  # botica: ignore[test-skip-reason] -- tests the marker itself
req.applymarker(pytest.mark.xfail)  # noqa: BTTEST012 -- tests the marker itself
```
