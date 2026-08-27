# Detection Reference

Full case catalog for `test-smell-review` (see `SKILL.md` for the protocol
that uses this file) — Python and TypeScript/JavaScript, plus a
language-agnostic path for anything else. Every code below is a condensed,
re-worded adaptation of a pattern documented in
[`vinicq/falsegreen-skill`](https://github.com/vinicq/falsegreen-skill)'s
`reference.md` (MIT license) and its companion static scanners
[`vinicq/falsegreen`](https://github.com/vinicq/falsegreen) (Python) and
[`vinicq/falsegreen-js`](https://github.com/vinicq/falsegreen-js) (TS/JS).
The upstream project also covers Robot Framework if a future repo needs
that — not adapted here, no target repo has asked for it yet.

**Confidence:** `HIGH` = report it, no plausible legitimate reading. `LOW` =
likely smell, note it but a legitimate reading can exist. `info` = opt-in
diagnostic only (maintainability, not a false-positive risk) — mention only
if asked, never as an unprompted finding.

**Structural vs. semantic:** codes marked "structural" are provable by a
parser — if `falsegreen` is installed, it already proved these (Step 0 of
`SKILL.md`); do not re-derive them by hand, just don't contradict them.
Codes marked "semantic" require reading the test's *intent* against the
production code or a spec — no parser can prove these; this is what the
in-context judgment pass in `SKILL.md` actually contributes.

---

## Family A — the test never checks anything (structural)

| Code | J | Pattern |
|---|---|---|
| C1 | J1 | `assert` inside an `if`/`for`/`while` whose branch/iterable may never execute — passes vacuously if never entered. Not flagged for a loop over a non-empty literal. |
| C2 | J1 | No assertion at all in the body (only `pass`/docstring/`...`/setup). Exempt: `@pytest.mark.skip`, `@pytest.mark.xfail(strict=True)`, `@given`/`@hypothesis` (generates its own checks). |
| C2b | J1 | Calls the SUT but the result is never asserted. Exempt if a called helper contains the real assertion, **or if the check is a fluent/library matcher rather than the bare `assert` keyword** — absence of `assert` is not absence of verification. Do not flag: `hamcrest.assert_that(x, equal_to(y))`, assertpy's `assert_that(x).is_equal_to(y)`, `numpy.testing.assert_allclose`/`assert_array_equal`, `pandas.testing.assert_frame_equal`/`assert_series_equal`, or a pytest-plugin meta-test's `result.assert_outcomes(...)`/`result.stdout.fnmatch_lines([...])` (the `pytester`/`testdir` API — this repo's own `tests/test_engine.py` doesn't use it, but a future scaffolding-CLI test suite well might). |
| C2c | J1 | `with self.subTest(...):` block that does work but asserts nothing. |
| C3 | J1 | `assert` inside a `try` whose `except` (bare or broad) swallows `AssertionError` with a `pass`/`continue` body. |
| C4 / C4b | J1 | Test function nested inside another function (pytest never collects it) / test class defines `__init__` (pytest skips it). |
| C20 | J1 | `assert` after an unconditional `return`/`raise`/`break`/`continue` — dead code, never runs. |
| C21 | J1 | Every assertion in the function is inside a conditional; none runs unconditionally. |
| C38 | J1 | Two `def test_*` share a name — Python silently binds the later over the earlier; the first never runs. |
| C39 | J1 | `return x == y` in a test — pytest ignores the return value, nothing is checked. |
| C43 | J1 | `pytest.skip()` mid-test, with real checks below it that then never run. |
| C45 | J1 | `@pytest.mark.parametrize(..., [])` — zero cases generated, the test never runs. |
| C49 | J1 | `pytest.warns`/`assertWarns` wraps more than one statement — an earlier line may warn while the target line never does. |
| C50 | J1 | `caplog`/`assertLogs` captures output but nothing reads or asserts it. |
| C51 | J1 | Empty-bodied `pytest.raises(...)`/`pytest.warns(...)` context — no call inside, so it can never fail. |
| C59 | J1 | `result == expected` as a bare statement — the boolean is computed and discarded. Use `assert`. |
| CC | J1 | A commented-out `# assert ...` line — the check was disabled and left. |

**Example (C2/C2b — the most common shape):**
```python
# BAD: calls the SUT, asserts nothing
def test_create_user():
    user = create_user("Alice")   # C2b — no assertion follows

# CLEAN
def test_create_user():
    user = create_user("Alice")
    assert user.name == "Alice"
```

## Family B — the check is weak or always true (structural)

| Code | J | Pattern |
|---|---|---|
| C5 | J2 | Always-true: `assert True`, `assert (a, b)` (non-empty tuple), `assert 1`. |
| C6 | J4 | Truthiness-only / `len(x) > 0` / substring-in-`str()` — checks presence, not the value. Exempt at the integration/E2E layer, where presence of a response *is* the check. |
| C6b | J3 | `mock.call_args.args[idx]` where `idx` is computed, not a literal — fragile positional access. |
| C6c | J4 | Bare `mock.call_count` truthiness — passes for any count ≥ 1, doesn't check *how many*. |
| C7 | J2 | Self-comparison: `assert x == x` — always true by reflexivity. Exempt if also checking `__eq__`/`__hash__` semantics against a distinct object. |
| C8 | J4 | Exact `==` on a float literal other than `0.0`/`1.0` — use `pytest.approx()`. |
| C8b | J4 | `assertAlmostEqual`/`pytest.approx()` with no explicit tolerance — the default can pass a meaningfully wrong value. |
| C9 | J4 | `pytest.raises(Exception)` (or no type) with no `match=` — any exception, including a typo in the test itself, satisfies it. |
| C11a | J2 | `obj.attr = X` immediately followed by `assert obj.attr == X` — only confirms Python's own attribute assignment. |
| C13 | J4 | `mock.assert_called_once` with no `()` — a bound-method attribute access that never runs. |
| C13b | J3 | `@patch(...)` without `autospec=True`/`spec=` — typos in call signature go undetected. |
| C14 | J2 | A "golden file" written from the test's own first-run output, then compared against forever. Exempt for intentional browser/UI snapshot testing. |
| C16 | J6 | Depends on `datetime.now()`/`time.time()` unfrozen, `time.sleep()`, or unseeded `random.*`. |
| C18 | J2 | `str(x)`/`repr(x)` compared to a literal — couples to formatting, not the value. |
| C25 | J1 | `@pytest.mark.xfail` without `strict=True` — an unexpected pass (XPASS) is silently accepted, not a failure. |
| C34 | J4 | Suboptimal form: `== True`/`== False`/`== None`/`not x in y` instead of `assert x`/`assert not x`/`is None`/`not in`. |
| C42 | J2 | `assert` on a generator expression or a bare lambda — always truthy as an object. |
| C44 | J2 | Numeric tautology: `len(x) >= 0`, `abs(x) >= 0`, `call_count >= 0` — always true. |
| C52 | J2 | `assert x in {x}` — the collection is built from the subject, membership is true by construction. |
| C55 | J3 | `assert m.foo == m.bar` where both sides are the test's own mocks, not the SUT. |
| C56 | J1 | `assert fetch_user()` where `fetch_user` is `async def` — asserts the (always-truthy) coroutine object, never its awaited value. |
| C57 | J3 | Expected side is `m.attr` on a bare `Mock()` with no `spec=` — attribute access auto-creates a fresh truthy mock. |

**Example (the two most common echo-shaped mistakes):**
```python
# BAD — C5: tautology
def test_active():
    assert True

# BAD — C7: self-comparison
def test_name():
    name = get_name()
    assert name == name

# BAD — C9: exception type too broad, plus a typo would still pass
with pytest.raises(Exception):
    devide(a, b)          # typo — NameError also satisfies "Exception"

# CLEAN
with pytest.raises(ZeroDivisionError, match="division by zero"):
    divide(a, 0)
```

## Family C — the test checks its own setup, not the program (structural)

| Code | J | Pattern |
|---|---|---|
| C19 | J1 | `with pytest.raises(E):` wraps more than one statement — if the first raises, the intended target line never runs. |
| C28 | J4 | `with pytest.raises(E) as exc:` but `exc` is never read afterward — checks the type only, not the message/attributes. |
| C29 | J6 | `os.environ["KEY"] = value` set directly in a test — persists across tests in the same process; use `monkeypatch.setenv()`. |
| C48 | J1 | Test flips a test-mode flag (`os.environ["TESTING"]`, `settings.TESTING = True`) then asserts — exercises the product's test-only branch, not real behavior. |

## Family D — green depends on external or shared state (structural)

| Code | J | Pattern |
|---|---|---|
| C17 | J1 | `pytest.skip()`/`skipTest()` inside a broad `except` — a real assertion failure triggers a skip instead of a fail. |
| C23 | J6 | Hard-coded absolute or home-relative file path — breaks in CI / on another machine. |
| C24 | J6 | Module-level mutable state (`list`/`dict`/`set`) mutated by one test and read by another with no reset — test order determines the outcome. |
| C27 | J1 | `try: sut_call() except: pass` with **no assertion inside the try at all** — success and failure both go green. (Different from C3, which wraps an actual assert.) |
| C30 | J3 | `responses.add(...)`/`httpretty.register_uri(...)` configured but the activator (`@responses.activate` etc.) is never applied — real HTTP goes through. |
| C31 | J4 | `capsys.readouterr()` called but the result is discarded or never asserted. |
| C32 | J1 | `@pytest.mark.skip` with no `reason=` — no explanation, easy to forget. |
| C35 | J6 | A `flaky`/`retry`/`rerun` decorator on the test — masks non-determinism instead of fixing it. |

## Family E — the test passes but checks the wrong thing (structural)

| Code | J | Pattern |
|---|---|---|
| C33 | J4 | An sklearn/ML metric (`accuracy_score`, `.score()`) is computed but the result is discarded, never asserted against a threshold. |
| C36 | J1 | `pytest.fail()` with no message — CI output shows only "FAILED", no context. |
| C37 | J2 | The exact same `@pytest.mark.parametrize` case appears twice — no additional coverage. |
| C41 | J4 | `assert not lst.sort()` — asserting on a mutator method that returns `None`; trivially satisfied regardless of the sort's correctness. |

## Diagnostic codes — opt-in, `info` only, never a finding unless asked

| Code | Pattern |
|---|---|
| D1 | Assertion Roulette: 2+ asserts, none with a message — a failure only names the line, not the condition. |
| D3 | Duplicate Assert: the identical assertion appears twice. |
| D4 | `@pytest.mark.parametrize` with 3+ cases and no `ids=` — CI shows `test[0]`, `test[1]`. |
| D5 | 5+ setup statements before the first assert — consider a fixture. |
| D6 | `print()` in the test body — usually a forgotten debug statement. Note: this repo's `log-no-print` (CES-46) explicitly excludes `tests/**`, so this diagnostic is the only place print-in-test is surfaced at all. |
| M2 | Test body over 50 lines — consider splitting into focused tests. |

---

# TypeScript / JavaScript

Runner-agnostic across Jest, Vitest, Mocha+Chai, Jasmine, AVA, `node:test`,
Cypress, Playwright, and Testing Library — the codes below apply regardless
of which one a project uses. Codes shared with Python keep the same `C`
number where the smell is the same concept (e.g. `C5` tautology, `C7`
self-compare); `JS*` codes are ecosystem-specific.

## Structural codes (provable by a parser — `falsegreen-js` proves these if installed)

| Code | Conf | Pattern |
|---|---|---|
| C2 | HIGH | Empty test body. |
| C2b | LOW | Calls the unit but never asserts. Exempt: supertest/chai-http's own `.expect()` (`request(app).get("/").expect(200)`) *is* the assertion at the API layer. |
| C5 | HIGH | Always-true: `expect(true).toBe(true)`, `assert(1)`. |
| C6 | LOW | Weak check: `toBeTruthy()`/`toBeDefined()`, `.length > 0`. |
| C7 | HIGH | Self-compare: `expect(x).toBe(x)`. |
| C8 | LOW | Exact equality on a float. |
| C9 | LOW | `toThrow()` with no error type or message. |
| C16 | LOW | Depends on `Date.now()`/`Math.random()`/a fixed timer, unfrozen/unseeded. |
| C18 | LOW | Stringified equality: comparing `String(x)`/`JSON.stringify(x)`/a template literal to a literal. |
| C20 | HIGH | Assertion in dead code after `return`/`throw`. |
| C21 | LOW | Every assertion in the test is conditional. |
| C23 | LOW | Reads a real file at a literal path, or a hard-coded URL (mystery guest). |
| C37 | LOW | Duplicate `it.each`/`test.each` case. |
| C44 | HIGH | Numeric tautology on a length: `expect(x.length).toBeGreaterThanOrEqual(0)`. |
| C48 | LOW | Dark patch: flips a test-mode flag (`process.env.NODE_ENV = "test"`) then asserts. |
| CC | LOW | Commented-out assertion. |
| JS1 | HIGH | Focused test (`it.only`/`fit`) — skips the rest of the suite. |
| JS2 | HIGH | `expect(x)` called with no matcher chained. |
| JS3 | LOW | A snapshot assertion is the *only* check. |
| JS4 | LOW | Skipped test (`it.skip`/`xit`/`it.todo`). |
| JS5 | LOW | An async query/event (`findBy*`, `waitFor`, user-event) is not awaited. |
| JS6 | HIGH | Empty `describe`/`suite` block. |
| JS7 | LOW | Assertion lives inside a non-awaited `setTimeout`/`.then()` callback. |
| JS8 | LOW | Mocks the unit under test and asserts the mock directly (the TS/JS shape of semantic case 10, but statically provable here). |
| JS9 | HIGH | Assertion inside a dead literal branch (`if (false) { ... }`). |
| JS11 | LOW | `try/catch` swallows the assertion. |
| JS13 | LOW | `queryBy*`/`queryAllBy*` (returns `null` when absent) used as a loose statement, never asserted. Not this code: `getBy*`/`getAllBy*`/`findBy*`/`findAllBy*` *throw* on absence, so they already are the assertion. |
| JS15 | LOW | A comparison wrapped in a boolean matcher: `expect(a === b).toBe(true)` instead of `expect(a).toBe(b)`. |
| JS17 | LOW | Commented-out test block (`// it(...)`). |
| JS18 | LOW | `done` callback used instead of async/await (easy to forget to call `done()` on a failing path). |
| JS21 | HIGH | A matcher is referenced but never called: `expect(x).toBe` with no `()`. |
| JS22 | HIGH | Empty `it.each`/`test.each` table — zero cases generated. |
| JS23 | HIGH | `expect.assertions(N)` declared with fewer unconditionally-reachable `expect()` calls than `N`. |
| JS24 | LOW | Cypress `cy.get/find/contains(...)` as a query statement with no `.should`/`.and`/`.then` assertion chained. |
| JS25 | HIGH | The only assertion sits inside an array-iterator callback (`forEach`/`map`/`filter`/`some`/`every`) — runs zero times on an empty collection. |
| JS26 | LOW | Fake timers installed but never advanced (`runAllTimers`/`advanceTimersByTime`) — the scheduled callback never fires. |
| JS27 | LOW | `toHaveBeenCalled*` is the sole oracle on a locally-created double — verifies wiring was called, not that it computed the right thing. |
| JS29 | LOW | `expect(...).resolves`/`.rejects` chain used as a bare statement, not awaited/returned — the test finishes green before the matcher settles. |
| JS30 | HIGH | Literal-vs-literal assertion: `expect(2).toBe(3)`, chai `expect(x).to.equal(y)` where both operands are fixed at parse time. |
| JS31 | LOW | `try/catch` swallows a possible throw with no assertion on the exception — a unit that stops throwing still passes green. |

**Diagnostic (opt-in, `info` only):** D1 assertion roulette, D3 duplicate
assert, D4 untitled `it.each` cases, D6 `console.*` in a test, D7 anonymous
test (blank/missing description), D8 magic number in an assertion, M2
over-long test body.

**Example (the two highest-prevalence patterns, per Jorge 2023's STEEL-tool
survey of 65 JS projects — 92.31% and 72.31% of projects respectively):**
```typescript
// BAD — C1-equivalent: assertion inside a branch that may never fire
it("validates items", () => {
    if (items.length) {
        expect(items[0].valid).toBe(true);   // never runs if items is []
    }
});

// BAD — C2b: calls the SUT, zero expect() calls anywhere in the body
it("processes the order", () => {
    const result = process(order);           // no expect() follows
});

// BAD — JS8 / semantic case 10: mocks the unit under test
jest.mock("./calculator");
import { add } from "./calculator";
(add as jest.Mock).mockReturnValue(5);
test("add", () => {
    expect(add(2, 3)).toBe(5);                // asserting the mock, not real addition
});

// CLEAN
test("add", () => {
    expect(add(2, 3)).toBe(5);                // real function, real arithmetic
});
```

---

## Semantic patterns — no parser can prove these; this is the skill's real value-add

These need reading the test against its stated intent and the production
code. Judgment only: never block a commit on one of these without showing
the reasoning, per `SKILL.md`'s HIGH/LOW rule. **Language-agnostic** — the
Python examples below translate directly to any other language: swap
`unittest.mock`/`@patch` for `jest.mock`/`sinon`/Go's interface-based fakes,
same judgment either way. This is also the section to apply for a language
with no structural table above (Go, Rust, Java, ...) — see "Applying this to
other languages" further down for worked examples in Go specifically.

- **Case 10 (J3, HIGH) — Mocks the unit under test.** Patches the very
  function/class under test, then asserts the mock's own configured value.
  ```python
  # BAD
  @patch('mymodule.add')
  def test_add(mock_add):
      mock_add.return_value = 5
      assert add(2, 3) == 5        # asserting the mock, not real addition
  ```
- **Case 11 (J2/J3, HIGH) — Asserts the value fed to the mock (an echo).**
  `stub.return_value = X; assert sut.method() == X` — the result passes
  through no production logic.
  ```python
  # BAD
  def test_price(mock_product):
      mock_product.price = 100
      assert get_price(mock_product) == 100   # echoes the stub
  ```
- **Case 12 (J2, HIGH) — Re-implements the production formula as "expected".**
  Both the test and the code compute `price + price * rate`; they agree on
  the same wrong answer if the formula itself is wrong.
  ```python
  # BAD
  def test_total():
      expected = 100 + 100 * 0.1        # re-derives the SUT's own formula
      assert calculate_total(100, 0.1) == expected
  # CLEAN — expected comes from the spec, not the formula
  def test_total():
      assert calculate_total(100, 0.1) == 110.0  # spec: 100 + 10% = 110
  ```
- **Case 15 (J6, HIGH) — Passes only if another test already ran.** Reads
  module-level state a sibling test wrote, with no reset between them.
- **Case 18 (J2, HIGH, adversarial-verified) — Expected value contradicts the
  spec.** The highest-stakes finding: it means a bug is frozen as "correct".
  **Never report without citing an independent oracle** (docstring, type
  annotation, explicit spec) — then argue the opposite side before reporting
  (see "Adversarial check" in `SKILL.md`).
- **S11 (J4, HIGH) — Negative-only security assertion.** A sanitizer/redactor
  test that asserts only `"secret" not in output`, with no paired positive
  assertion that legitimate content survived — passes even if the whole
  output was wrongly dropped. Exempt if the filter's actual contract is to
  drop the input entirely (a blocklist that should return empty).
- **S12 (J3, LOW) — Patches core logic, not an external edge.** Deeper than
  case 10: patches a private method or a direct collaborator *of the class
  under test*, not a genuine external edge (DB/network/clock — those are
  fine). Ask: is the patched thing an edge, or the unit's own core behavior?
- **S16 (J4, LOW) — Call-verification as the sole oracle.** `mock.save.assert_called_once()`
  with no assertion on the SUT's own return value/state — passes even if the
  SUT computed the wrong thing before delegating. Exempt when a call-with-args
  assertion (`assert_called_once_with(...)`) is paired with a result assertion.
- **S17 (J4, HIGH) — Exception-path oracle blindness.** `pytest.raises(Exception)`
  (or no type) where the *actual* raise came from a typo in arrange, not the
  SUT — the test never really reached the code path it claims to verify.
  ```python
  # BAD — a typo raises before the SUT line is ever reached
  def test_withdraw_over_limit():
      acct = Acount(balance=10)          # typo: NameError, not the SUT's error
      with pytest.raises(Exception):     # "passes" for the wrong reason
          account.withdraw(50)
  ```
- **S1 (J4) Intent mismatch** — the name/docstring claims to verify X, the
  assertion checks Y. **S5 (J3) Tests the framework** — the assertion proves
  a language/library guarantee (a dict stores a key), not the unit's logic.
  **S21 (J2, LOW) Self-judging LLM assertion** — the oracle is *another*
  model call (`assert judge_llm(...) == "yes"`), sharing the same blind spots
  as the thing being judged.

Full detail on every one of these, plus Robot Framework coverage this repo
doesn't currently need: see falsegreen-skill's own `reference.md` —
https://github.com/vinicq/falsegreen-skill/blob/master/reference.md.

---

## Applying this to other languages (Go, Rust, Java, ...)

No dedicated structural catalog exists for these — falsegreen's own family
doesn't cover them either, so there's nothing upstream to adapt from. That
does **not** mean skip the review: J1–J6 (`SKILL.md` Step 3) and the semantic
patterns above are judgments about what a test *does*, not about a specific
language's grammar, and they transfer directly. Two worked Go examples,
written for this adaptation (not from falsegreen — flagging that distinction
explicitly, per this file's own citation standard):

```go
// BAD — J2/case 12: re-implements the production formula as "expected"
func TestTotal(t *testing.T) {
    price, rate := 100.0, 0.1
    expected := price + price*rate     // re-derives the SUT's own formula
    if got := CalculateTotal(price, rate); got != expected {
        t.Errorf("got %v, want %v", got, expected)
    }
}
// CLEAN — expected comes from the spec, not the formula
func TestTotal(t *testing.T) {
    if got := CalculateTotal(100.0, 0.1); got != 110.0 { // spec: 100 + 10% = 110
        t.Errorf("got %v, want 110.0", got)
    }
}

// BAD — J3/case 10: mocks the interface under test, asserts the mock's own value
type mockAdder struct{ result int }
func (m mockAdder) Add(a, b int) int { return m.result }
func TestAdd(t *testing.T) {
    m := mockAdder{result: 5}
    if got := m.Add(2, 3); got != 5 {   // asserting the mock, not real addition
        t.Errorf("got %v, want 5", got)
    }
}
// CLEAN — exercises the real implementation
func TestAdd(t *testing.T) {
    if got := Add(2, 3); got != 5 {
        t.Errorf("got %v, want 5", got)
    }
}
```

The same judgments apply to `testify`'s `assert.Equal`/`require.Equal`, a
Rust `#[test]` with `assert_eq!`, or a JUnit `@Test` — J1 ("does the
assertion run") still means "not dead code after an early `return`/`panic!`/
exception," J7-equivalent self-comparison is still `assert_eq!(x, x)`,
table-driven subtests (`t.Run(...)` in Go) still need J6 (no shared mutable
state between subtests). If a language gets its own falsegreen-family
scanner in the future, add a structural table for it here the same way the
TS/JS section above was added — don't wait for that to apply the judgments
by hand today.

---

## Credits and references

- **[`vinicq/falsegreen-skill`](https://github.com/vinicq/falsegreen-skill)**
  (MIT) — source of the J1–J6 judgment framework and this entire case
  catalog (Python and TS/JS sections above). This file is a condensed,
  re-worded adaptation credited here per its own license terms; the
  original is far more complete (Robot Framework, the full JS-series,
  semantic exemption lists, authoring/fix modes). The Go examples in
  "Applying this to other languages" above are **not** from falsegreen —
  they're original to this adaptation, since no upstream Go catalog exists
  to credit.
- **[`vinicq/falsegreen`](https://github.com/vinicq/falsegreen)** (MIT) — the
  deterministic Python/pytest AST scanner that proves the structural half of
  the Python catalog for free, no LLM needed. Proposed separately as a house
  prek hook in [scaffolding#124](https://github.com/collectiveai-team/scaffolding/issues/124).
- **[`vinicq/falsegreen-js`](https://github.com/vinicq/falsegreen-js)** (MIT)
  — the TS/JS companion scanner; proves the structural half of the TS/JS
  catalog above the same way.
- **falsegreen's own `CREDITS.md`** cross-walks this taxonomy against the
  academic literature: Elvys Alves Soares, *A Multimethod Study of Test
  Smells: Cataloging, Removal, and New Types* (PhD thesis, UFPE, 2023) — the
  source of the "rotten green test" concept; Julien Delplanque et al.,
  *Rotten Green Tests*, ICSE 2019 — the term's origin; Tongjie Wang et al.,
  *PyNose: A Test Smell Detector for Python*, ASE 2021 (JetBrains Research)
  — the closest prior academic Python-specific tool; Dalton Nicodemos Jorge's
  2023 UFCG thesis and the STEEL tool — the source of the TS/JS conditional-
  assertion and zero-assertion prevalence figures cited above.
- **[The Open Catalog of Test Smells](https://test-smell-catalog.readthedocs.io/)**
  — a community-maintained, 517-smell aggregator falsegreen itself
  cross-walks against; the reference to fall back on if a future gap isn't
  in this file's scope.
