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
- job "test" in ".github/workflows/ci.yml": blake3 9cf703488da371253eaf3fa2219331bde147f6b1f215bfe156218337981d12f4
<!-- /botica:covered -->
