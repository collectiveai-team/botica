---
role: tests
covered:
  hooks:
    - file: prek.toml
      id: botica
  jobs:
    - file: .github/workflows/ci.yml
      id: test
---

## Invariants

- The tests run in the job `test` of `ci.yml`.

<!-- botica:covered -->
- hook "botica" in "prek.toml": blake3 9ef3d66ecf12f1d092fb203bc7d1032d888be8ff096d1853300c59f0d561448d
- job "test" in ".github/workflows/ci.yml": blake3 b6a8efa542737de1fd10109b1777e3ff11896b0587bdca345826a617c84d76d8
<!-- /botica:covered -->
