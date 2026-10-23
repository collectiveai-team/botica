## Directive

The configuration of pytest in `pyproject.toml` makes a run fail on a marker that `markers` does
not register, on a setting that pytest does not know, on an `xfail` test that passes, on a
`parametrize` with no parameter set or with two sets of the same id, and when pytest-timeout is not
installed. An error by default.

## Why

Without these settings each of those mistakes leaves the run green: a typo in a marker moves a test
to the wrong job, a typo in a setting drops the setting, a fixed bug stays marked `xfail`, an empty
`parametrize` skips its test, two parameter sets with the same id report as one, and a `timeout`
with no pytest-timeout installed is ignored.

## Tool

[pytest](https://docs.pytest.org/en/stable/reference/reference.html#configuration-options) 9.0 or
later reads the settings from `[tool.pytest.ini_options]`. `external-config` reads them where
pytest does (`pytest.toml`, `pytest.ini`, `[tool.pytest]` or `[tool.pytest.ini_options]` of
`pyproject.toml`, `tox.ini`, `setup.cfg`) and reports the settings that lack them.

## Configuration

```toml
[tool.pytest.ini_options]
minversion = "9.0"
strict_config = true
strict_markers = true
strict_xfail = true
strict_parametrization_ids = true
empty_parameter_set_mark = "fail_at_collect"
required_plugins = ["pytest-timeout"]
timeout = 60
session_timeout = 900
```

`external-config` checks the six settings of its fragment. It does not check `minversion`,
`timeout` or `session_timeout`, whose value a repository may change, or `strict = true`, which
turns on the same settings: a repository that uses it, or a version in `required_plugins`
(`"pytest-timeout>=2.4"`), writes the keys of the fragment as they are, or switches this rule off.
`required_plugins` may name more plugins. pytest 9 renamed `xfail_strict` to `strict_xfail` and
reads the old name too, but `external-config` does not: a repository with `xfail_strict = true`
renames it to `strict_xfail = true`.

## Do

```python
@pytest.mark.integration
def test_the_order_is_saved(database): ...
```

## Don't

```python
@pytest.mark.integraton
def test_the_order_is_saved(database): ...
```

## Suppression

`@pytest.mark.timeout(N)` gives a slow test its own limit, and
`@pytest.mark.xfail(strict=False, reason="...")` keeps a test that passes only sometimes. The rule
itself is switched off with `ignore = ["pytest-strict"]` in the configuration of botica.
