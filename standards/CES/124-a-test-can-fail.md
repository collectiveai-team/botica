---
id: CES-124
title: A test can fail
---

## Directive

A test fails when the behaviour it checks breaks. Its assertions run on every pass, compare the
result with a value that the test writes down, and no `try` or `except` in the test catches their
failure. A test that only calls the code and asserts nothing is a smoke test, and its name says so.

## Why

- A test that cannot fail reports a broken program as working. It also raises the coverage of the
  lines it runs, so the gap that CES-66 reads is hidden.
- A model asked to make a suite pass writes these tests when it does not know the expected value:
  `assert True`, a value compared with itself, an assertion inside `try` with `except: pass`.

## How it is checked

| Case | Checked by |
|---|---|
| a value compared with itself, a constant asserted (Python, TypeScript, JavaScript, Go, Rust) | `no-tautological-assert` |
| a tuple or a string asserted, a name compared with itself, two constants compared, a comparison whose result is dropped, a `pytest.raises` too broad or around more than one statement | ruff `F631`, `PLW0129`, `PLR0124`, `PLR0133`, `B015`, `PT011`, `PT012` of `base-python` |
| a test with no assertion, an assertion that a `try` swallows, a test that returns a comparison, an uncollected test, a mock assertion with a typo (pytest) | the hook `falsegreen` of the recipe `python-test-smells`, before each commit |
| several asserts without a message, the same assert twice, a long setup before the first assert, a `print()` in a test, a long test (pytest) | the same hook, as information that does not fail it (`D1`, `D3`, `D5`, `D6`, `M2`) |
| an expected value copied from the output, a mock of the unit under test | review, with the skill `test-smell-review` |

## Adoption in an existing repository

`uvx falsegreen@0.9.2 tests/ --write-baseline` writes `.falsegreen-baseline.json` with the findings
there are, and the hook then fails only on new ones. The file is committed, and an entry is
removed when its test is fixed.
