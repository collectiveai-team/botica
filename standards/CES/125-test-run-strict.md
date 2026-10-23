---
id: CES-125
title: A test run fails on what it would otherwise pass over
---

## Directive

The configuration of pytest makes a run fail, instead of passing, on each of these:

- a marker that the `markers` setting does not register (`strict_markers`);
- a setting that pytest or its plugins do not know (`strict_config`);
- a test marked `xfail` that passes (`strict_xfail`);
- a `parametrize` with no parameter set (`empty_parameter_set_mark = "fail_at_collect"`);
- two parameter sets with the same id (`strict_parametrization_ids`);
- a plugin that the configuration needs and the environment lacks (`required_plugins`);
- a test that runs longer than its limit (`timeout`, from pytest-timeout);
- a run that goes on longer than its limit (`session_timeout`, from pytest-timeout).

The settings go in `[tool.pytest.ini_options]` of `pyproject.toml`, with `minversion = "9.0"`.

## Why

Each of these mistakes leaves the run green. A typo in a marker (`@pytest.mark.integraton`) puts an
integration test in the run of the unit tests, and `-m "not integration"` deselects nothing. A typo
in a setting (`filter_warnings`) drops the setting. A test that loops forever runs until the job of
CI stops it, after 6 hours on GitHub Actions, while pytest-timeout, installed and with no `timeout`,
does nothing. A `parametrize` over a list that became empty skips the test, and an `xfail` that
started to pass hides that the bug is fixed. With these settings, each one fails the run and names
the test or the setting.

## Settings

```toml
[tool.pytest.ini_options]
minversion = "9.0"
addopts = ["-ra"]
strict_config = true
strict_markers = true
strict_xfail = true
strict_parametrization_ids = true
empty_parameter_set_mark = "fail_at_collect"
required_plugins = ["pytest-asyncio", "pytest-timeout"]
timeout = 60
session_timeout = 900
```

The `strict_*` settings exist from pytest 9.0. pytest 8 does not know them and only warns, so
`minversion = "9.0"` makes pytest 8 stop instead. pytest 9.0 ignores `--strict-markers` and
`--strict-config` in `addopts` ([pytest#14442](https://github.com/pytest-dev/pytest/issues/14442),
fixed in 9.1), so the settings are keys and not options in `addopts`. `strict = true` turns on the
same settings and the ones that later releases of pytest add; the keys name each one.

`timeout` stops one test after 60 seconds. `session_timeout` stops the run after 900 seconds:
pytest-timeout checks it when a test ends, so it does not stop a test that hangs, which is what
`timeout` is for. A run that reaches it fails with `session-timeout: 900.0 sec exceeded`.

`-ra` prints the reason of each skipped, xfailed and failed test at the end of the run.

## Migration (existing repos)

1. `strict_markers`: register each marker that the tests use, or fix its name.
2. `strict_config`: fix or delete each setting that the error names.
3. `timeout`: give a slow test `@pytest.mark.timeout(N)`, or raise `timeout`.
4. `session_timeout`: raise it when the suite takes longer than 900 seconds.
5. `strict_xfail`: rename `xfail_strict = true`, its name before pytest 9, to `strict_xfail = true`,
   and delete each `xfail` that now passes.
6. `required_plugins`, `strict_parametrization_ids`, `empty_parameter_set_mark` and `-ra`.

## Suppression

A test that takes longer than the limit takes `@pytest.mark.timeout(N)`. A test that is expected to
pass sometimes takes `@pytest.mark.xfail(strict=False, reason="...")`.
