---
name: test-smell-review
description: Review pytest/unittest, Jest/Vitest/Mocha, or any other language's tests for false-green smells — tests that pass without protecting anything (empty, assertion-free, tautological, mocking the unit under test, echoing a mock back to itself). Use after writing or editing test files, and when the user asks to review test quality, find weak tests, or check whether tests actually catch bugs.
---

# Test Smell Review

A test suite that is 100% green is not proof the code is correct — it is only
proof that no test failed. Apply this judgment whenever tests are written,
edited, or explicitly reviewed: **a test is useful only if some incorrect
implementation would make it fail.** If no such implementation exists, the
test is structurally green regardless of whether the code is right.

## Why this is a native skill, not the upstream CLI

The obvious approach — installing
[`falsegreen-skill`](https://github.com/vinicq/falsegreen-skill) (the
upstream npm package this is adapted from) and shelling out to it — is a poor
fit for an agentic coding session:

- It makes a **second, separate LLM API call** per file (its own
  `ANTHROPIC_API_KEY`/`OPENAI_API_KEY`/`GEMINI_API_KEY`, its own cost, and
  non-deterministic output on top of the in-context judgment the current
  agent session can already make for free).
- Its install paths (Claude Code marketplace plugin, Antigravity/Gemini
  extension manifests, a Cursor `.mdc` rule file, npm global/dev install)
  don't map onto this repo's own `.agents/skills/<slug>/SKILL.md`
  convention — the one `agent_config.py` already wires identically across
  opencode/claude-code/codex.
- It's a second runtime (Node 18+) in a Python-only tool.

This skill extracts the **protocol** (the J1–J6 judgments and the case
catalog) as native, in-context instructions instead — see `reference.md` for
the full catalog, condensed and credited from the same source, and the
"Credits and references" section there for the complete citation chain
(academic sources included). No subprocess, no API key, no Node — the agent
running this skill applies the judgment itself.

## If you're about to write a test, not just review one

This skill is analysis-only (the upstream's "Mode A"). It deliberately does
not port the upstream's test-authoring mode — that would duplicate this
repo's existing `tdd` skill (red-green-refactor workflow, mocking guidance).
Instead, borrow one discipline from it when writing a test, then self-check
with Steps 1–4 below before calling it done: **derive the expected value from
an independent source — the spec, a docstring, a hand-computed value — never
from running the code once and copying its output.** A test built that way
is a characterization test, and characterization tests can't fail when the
code is wrong; that is exactly what this skill exists to catch.

## Step 0a: detect the language and framework

- **Python:** `import pytest`/`import unittest`/`from unittest.mock import`,
  `@pytest.mark.*`, `@patch`, plain `assert`/`self.assert*`.
- **TypeScript / JavaScript:** `import { describe, it, expect } from ...`
  (`@jest/globals`, `vitest`), `require('chai')`, global `describe()`/`it()`/
  `test()`/`expect()`, mock cues `jest.fn()`/`vi.fn()`/`sinon.stub()`.
  Runner-agnostic across Jest, Vitest, Mocha+Chai, Jasmine, AVA, `node:test`,
  Cypress, Playwright, and Testing Library — `reference.md`'s TS/JS section
  applies to all of them.
- **Any other language (Go, Rust, Java, ...):** no dedicated structural
  catalog exists here (see `reference.md`'s "Applying this to other
  languages" section) — the language-agnostic J1–J6 judgments (Step 3) and
  the semantic cases (Step 2) still apply directly. Do not skip the review
  just because there's no matching table of codes.

## Step 0b: run the deterministic scanner first, if installed

Python's companion scanner is
[`falsegreen`](https://github.com/vinicq/falsegreen); TS/JS's is
[`falsegreen-js`](https://github.com/vinicq/falsegreen-js). If either is
available, run it on the changed test files before applying any judgment
below — it proves the purely structural cases (empty test, no assertion,
tautological assert, swallowed exception, sleep-in-test, and more — the
"structural"-labeled rows in `reference.md`) deterministically, for free,
with no LLM involved:

```bash
# Python — no install needed, matches this house's uv convention
uvx falsegreen tests/test_changed_file.py

# TypeScript / JavaScript — no install needed
npx falsegreen-js tests/test_changed_file.test.ts
```

Everything either scanner flags is already proven by a parser; do not
re-derive those findings by hand, and do not contradict them. Apply the
judgment protocol below to what a parser cannot prove — that is what the
rest of this skill is for. (`falsegreen` itself is proposed as a house prek
hook separately —
[scaffolding#124](https://github.com/collectiveai-team/scaffolding/issues/124)
— so it may not be installed yet; if it isn't, apply `reference.md`'s
structural catalog by hand instead of skipping it.)

If a `[tool.falsegreen]` block exists in `pyproject.toml` (or `.falsegreen.toml`;
check for an equivalent JS-side config too), read its `exclude`/`disable`/
`severity` settings before judging — that project has already declared some
patterns as intentional (a custom assertion helper, a layer override, a code
turned off) for the deterministic scanner. Honor the same exclusions here
rather than re-flagging what the project already decided isn't a smell for
it. Do not invent a second, skill-only config file for this — one config
surface, shared with the scanner.

## Step 1: classify the test's intent

Before judging the expected value, classify the test. Misclassifying here is
the most common cause of a false alarm:

| Class | Meaning | What counts as the oracle |
|---|---|---|
| spec / TDD | the test *is* the spec; code must match it | the test itself |
| characterization | deliberately freezes today's behavior (e.g. during a refactor) | the current output, by design |
| regression | records a specific known bug fix | the bug report |
| behavior | verifies a production rule or contract | the spec, docstring, or types |

A failing spec/TDD test is not a false positive. A labeled characterization
snapshot is not a frozen bug — do not flag case 18 or C14 against one. Ask
the user if the intent isn't stated in the test name, docstring, or a nearby
comment and it materially changes the verdict.

## Step 1b: read the test level — do not guess

Several codes (C6, C9, C14, and the semantic cases) are only judged correctly
if the level is right. Read it from signals, do not assume "everything here
is a unit test." Precedence, strongest signal wins:

1. **A doubled boundary beats an import.** Python: `unittest.mock`/`patch`/
   `monkeypatch`/`pytest-mock`/`responses`/`requests-mock`/`httpretty`/
   `respx`/`moto`/`fakeredis` intercepts the boundary. TS/JS:
   `jest.mock`/`vi.mock`/`jest.fn`/`vi.spyOn`/`sinon`, `msw`, `nock`. Either
   way, the test is unit/component **even if** a real client (`requests`,
   `boto3`, `axios`, SQLAlchemy, Prisma) is imported — the mock *is* the
   boundary.
2. **Else a real boundary makes it integration.** Python: an in-process test
   client (FastAPI `TestClient`, Flask `test_client`, Django `Client`), a real
   ORM session against a real (even ephemeral/containerized) database. TS/JS:
   supertest `request(app)`, Nest `Test.createTestingModule` + supertest, a
   real DB driver/ORM (Prisma, TypeORM, `pg`, `mongoose`) with no double.
   Either way: a real queue/cache/storage client with no double, still
   integration.
3. **Else a browser/mobile driver makes it E2E** (Playwright, Selenium,
   Cypress, WebdriverIO, Appium — same signal set regardless of language).
4. **No signal → unit.** Real, undoubled I/O in a test with no integration
   signal is itself the smell (mystery guest / over-mocking-inverted, J3/J6)
   — not a legitimate integration test that happens to lack a marker.

Why it matters concretely: a bare truthiness check on a response object (C6)
is a weak-check finding at unit level but *is* the real check at
integration/E2E, where the response existing is the point. Getting the level
wrong in either direction produces a wrong verdict, not just a miscategorized
one.

## Step 2: apply the case catalog (see `reference.md`)

`reference.md` in this skill's directory has the full catalog, organized by
language: Python (Family A–E structural codes), TypeScript/JavaScript (its
own structural table), and one shared "Semantic patterns" section that
applies to **every** language, including ones with no dedicated table — load
the section matching what Step 0a detected. Walk it deliberately — do not
rely on memory of "the famous ones" (assertion-free, tautology) and skip the
rest. In particular, always screen for the semantic cases 10/11/12/15/18 and
at least S11/S12/S16/S17 — these are exactly what a static scanner (Step 0b)
cannot catch, and are the actual reason this skill exists on top of it.

## Step 3: apply the six judgments (J1–J6)

For each test, ask these in order and stop at the first one that fails; every
code in `reference.md` maps to one of these:

| # | Question | Catches |
|---|---|---|
| J1 | Does the assertion actually run? | Dead code after `return`, a loop over an empty collection, an exception swallowed before the assert |
| J2 | Is the expected value from an independent source, not the code's own output? | Asserting a mock's `return_value` back at itself (an echo), re-deriving the production formula as the "expected" value |
| J3 | Is the real unit under test — not a mock standing in for it? | Patching the function being tested instead of one of its dependencies |
| J4 | Does the assertion verify enough, and the right thing? | Truthiness-only checks, `len(x) > 0`, a `str()`/`repr()` comparison that checks formatting instead of the value |
| J5 | Is the test coupled to implementation internals rather than behavior? | Asserting on a mock's positional call args by a computed index, testing a private method directly |
| J6 | Does the test pass in isolation, independent of run order? | Shared mutable module-level state, a test that only passes because an earlier test happened to run first |

Flag **HIGH** only when the failed judgment has no plausible legitimate
interpretation. Flag **LOW** when the smell is likely but a legitimate
reading exists. Everything else is **PASS** — say nothing about it.

**Severity is a ceiling, not a floor.** The catalog's listed severity is the
maximum for that code. Step 1's intent classification can only lower it (a
HIGH code on a deliberate characterization/spec test drops to LOW or is
withdrawn) — it never raises a code above its catalog severity.

## Step 4: adversarial check for case 18 (and any other spec-contradiction finding)

Case 18 — "the expected value contradicts the spec" — is the highest-stakes
finding: it means a bug may be frozen as "correct". Before reporting it:

1. Cite the independent oracle by name (docstring line, type annotation,
   explicit spec section, domain rule). If you cannot cite one, do not
   report it — that's a call for a human, not something to assert from
   pattern-matching alone.
2. Argue the opposite side: assume the expected value is correct, and try to
   defend it. If a plausible defense holds, withdraw the finding or downgrade
   to LOW.
3. Report HIGH only when the cited oracle clearly contradicts the value and
   the adversarial argument doesn't hold.

## Precision rules (do not over-flag)

- Boolean predicates (`isinstance(...)`, `.exists()`, `.is_dir()`) are real
  assertions, not weak truthiness checks (not C6).
- A fluent/library matcher (`hamcrest.assert_that`, assertpy, `numpy.testing.*`,
  `pandas.testing.*`) is the real check even with no bare `assert` keyword —
  absence of the keyword is not absence of verification (not C2/C2b; full
  exemption list in `reference.md`).
- A mock replacing a genuine external edge (DB, network, clock) is never
  case 10/S12; those apply only when the mock replaces the unit being tested.
- `@given`/`@hypothesis`-decorated tests with no explicit `assert` are not
  C2 — the framework generates and checks its own assertions.
- **Tie-break:** when more than one code could fire on the same
  `pytest.raises(...)` (e.g. C9 broad-type, C28 unread binding, S17
  exception-path blindness), report only the most specific one — a broad
  `pytest.raises(Exception)` followed by an assertion on the bound message
  satisfies all three at once; don't stack findings for one root cause.

## Step 5: report

For each real finding, give: the file and line, the code + judgment that
failed, confidence, a one-line reason, and a concrete fix — an independent
expected value or a narrower assertion, not just "add more asserts":

```
tests/test_discount.py:14  [C11 / J2] HIGH
  Asserts mock_rate.return_value back at itself — passes for any return
  value, including a wrong one.
  Fix: assert the real computed result, e.g. `assert result == 90` (10% off
  100, from the spec), not `assert result == mock_rate.return_value`.
```

Then a one-line summary: `Tests reviewed: N. Findings: M (H high, L low).`

If 3 or more findings share the same code, add one closing line suggesting
the user note it as a project convention if the pattern is actually
intentional there — don't repeat the note for every occurrence.

## Credits and references

Full attribution, the complete catalog (Python and TypeScript/JavaScript),
and the academic sources this protocol traces to (Soares 2023's "rotten
green test" thesis, Delplanque et al. ICSE 2019, PyNose/JetBrains Research
ASE 2021, Jorge 2023's STEEL tool, and the Open Catalog of Test Smells) are
in `reference.md`, which this skill loads alongside itself — read it, do not
skip straight to judgment from this file alone.
