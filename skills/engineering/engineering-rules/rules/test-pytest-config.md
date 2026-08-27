---
title: Use pytest with the shared ini_options baseline
section: test
scope: general
applies-to: all
status: current
tags: test, pytest, configuration
---

## Use pytest with the shared ini_options baseline

pytest is the test runner for every repo. The shared `[tool.pytest.ini_options]` block
declares `testpaths` and a `filterwarnings` triple that turns warnings into errors except
for `UserWarning` and any `DeprecationWarning`. Keep both fields intact — removing them
silences regressions or causes unrelated warnings to break CI.

**Prefer:**

```toml
# pyproject.toml
[tool.pytest.ini_options]
testpaths = ["tests"]
filterwarnings = [
    "error",
    "ignore::UserWarning",
    'ignore:.*:DeprecationWarning',
]
```

**Avoid:**

```toml
# Omitting filterwarnings lets deprecation noise accumulate and masks real errors.
[tool.pytest.ini_options]
testpaths = ["tests"]
```

The `testpaths` value varies by repo (`tests` or `test`) — preserve the one declared.

Reference: see `test-coverage-gap` for the baseline testing expectations.
