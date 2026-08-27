# CES-111 · test order randomization (pytest-randomly)

**Code:** `CES-111` &nbsp;·&nbsp; **Slug:** `test-order-randomization-pytest-randomly`
&nbsp;·&nbsp; **Enforced by:** dev dependency (pytest plugin) &nbsp;·&nbsp; **Tracker:**
[#111](https://github.com/collectiveai-team/scaffolding/issues/111)

## Directive

Test suites must not depend on execution order.
[`pytest-randomly`](https://github.com/pytest-dev/pytest-randomly) ships in the `dev` dependency
group; it activates automatically as a pytest plugin on every `pytest` invocation (local, prek,
CI) — no separate hook or CI step is needed.

## Why

Order-dependent/state-leaking tests are a classic latent bug class that gets *more* likely as more
independently-scoped AI-agent sessions add tests to the same suite over time — each session sees
only local context and has no visibility into shared fixtures or module state another session's
tests might leave dirty. `pytest-randomly` catches this for free.

## When a test fails under randomization

The failure output prints the seed (`Using --randomly-seed=1234`), so the failure is fully
reproducible: `pytest -p randomly --randomly-seed=1234`. Fix the underlying coupling (a shared
mutable fixture, module-level state, a test that depends on a previous test's side effect) — this
is a real bug the previous fixed order was hiding, not a false positive.

## Migration (existing repos)

Adding this may immediately surface pre-existing order-dependency failures in the current suite.
Treat those as real bugs to fix. As a last resort during triage, pin `-p no:randomly` on a
*specific* CI job while the underlying coupling gets fixed — never silently, and never as a
permanent state.

## Suppression

There is no per-test opt-out; `-p no:randomly` disables the plugin suite-wide for one invocation
and should only ever be a temporary triage step, tracked back to the coupling bug it's masking.
