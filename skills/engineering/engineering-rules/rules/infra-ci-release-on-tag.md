---
title: CI builds and releases on v* tags; run pytest where it exists
section: infra
scope: general
applies-to: all
status: current
tags: ci, github-actions, release, pytest, infra
---

## CI builds and releases on v* tags; run pytest where it exists

CI releases are tag-triggered on `v*` patterns. Python packages build a wheel + sdist and
upload to a GitHub Release. Service repos build Docker images to a container registry and
SSH-deploy. Repos that have tests run `pytest` in CI — either on every push/PR or as part
of the tag workflow. Repos with no CI should add at least a lint/test workflow.

**Prefer (tag-triggered build + release; pytest on push/PR where tests exist):**

```yaml
# python library
on:
  push:
    tags:
      - "v*"

jobs:
  build-and-release:
    steps:
      - run: uv build
      - run: gh release create ${{ github.ref_name }} dist/*.whl dist/*.tar.gz

# service repo
on:
  push:
    branches: [main, staging, dev]
    tags: ["v*.*.*"]
  pull_request:
    branches: [main, staging, dev]

jobs:
  test:
    steps:
      - run: uv sync --all-groups
      - run: uv run pytest
```

**Avoid:**

```yaml
# Don't skip tests in CI even when a tag triggers a release build
on:
  push:
    tags: ["v*"]
# (no test job — don't ship without running tests if the repo has them)
```

Reference: see `infra-docker-first` for the image structure built in CI.
