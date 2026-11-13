---
role: tests
applies:
  paths: []
  tiers: [low, medium, high]
stages: [local, ci]
blocking: P1
owners: ["@jedzill4"]
max-findings: 10
---

## Invariants

- The Rust tests run in the job `fmt, clippy & test` of `ci.yml` (`cargo test --workspace --locked
  --no-fail-fast`), the Python tests in the job `lint, type-check & test` of `tests.yml`
  (`uv run pytest`), the Go SDK in the step `Test the Go SDK` of `ci.yml`, and the code of the
  topics of `catalog/references/` in the job `code of the topics` of `ci.yml`
  (`scripts/check_references.sh`). On a pull request, `ci.yml` runs only when a changed file
  matches its `paths` filter.
- The variables `BOTICA_REQUIRE_RUFF`, `BOTICA_REQUIRE_READ_ERRORS`, `BOTICA_REQUIRE_NODE` and
  `BOTICA_REQUIRE_WORKFLOW_LINTERS` in the `env` of `ci.yml` turn a test that skips itself when a
  tool is missing into a failure. The Rust tests read them; `tests.yml` sets none. A new Rust test
  that skips itself when a tool is missing reads a switch of this kind, and `ci.yml` sets it.
- `[tool.pytest.ini_options]` of `pyproject.toml` keeps pytest strict (`strict_config`,
  `strict_markers`, `strict_xfail`, `strict_parametrization_ids`,
  `empty_parameter_set_mark = "fail_at_collect"`, `required_plugins`) and the tests off the network
  (`--disable-socket`). No `conftest.py` of `tests/` gives a test the network; the `conftest.py` of
  `catalog/references/prefect/tests/` gives it to the tests marked `integration` or `e2e`.
- The test `conformance.rs` of `botica-cli` runs each rule of the catalog that has a deterministic
  implementation over its fixtures, `catalog/rules/<family>/<rule>/fixtures/`, compares the
  findings with their `expected.json`, and checks the list of those rules by name. It skips the
  judgment and external rules and the rules whose status is `removed`, and some rules have no
  fixtures.
- Tests reach the logic through in-memory adapters of the ports of the project and through the
  interface that production code calls (`botica refs show in-memory-adapters`).

## Check always

- A test that is deleted, marked `#[ignore]`, `skip`, `xfail` or flaky, or loosened, and the
  behaviour of the code it tested.
- An entry removed from `expected.json`, a fixture removed, or a rule removed from the list of
  `conformance.rs`, when the rule itself stays.
- A path removed from the `paths` filter of `ci.yml`, a step removed from the jobs above, a hook
  removed from `prek.toml`, or `|| true` or `continue-on-error` added to a step that runs tests.
- A `BOTICA_REQUIRE_*` variable removed from `ci.yml`, or a test that skips itself when a tool is
  missing with no such variable.
- A strict setting or `--disable-socket` removed from `[tool.pytest.ini_options]`, and a marker or
  hook that gives a test the network (`enable_socket`, `allow_hosts`, a `conftest.py` that adds
  them).
- `test-coverage-gap` and `coverage-exclusion-reachable`: their `when` is `pull-request`, so the
  rubric of `abide.yml` leaves them out and this role reviews what they cover.
- A new behaviour of a crate with no test under `crates/<crate>/tests/` or in a `#[cfg(test)]`
  module that fails when the behaviour is wrong.

## Do not report

- The content of the fixtures under `catalog/rules/**/fixtures/` and
  `crates/botica-complexity/tests/fixtures/` judged as tests: they are wrong on purpose, and they
  are test data. A fixture or an `expected.json` that the diff removes or changes is still
  reviewed under `## Check always`.
- That a test under `crates/*/tests/` starts the `botica` binary: that is the interface of the
  command line. Its assertions are still reviewed.
- A rule of the family `testing` that the configuration of `[tool.botica]` on the target leaves
  out. A rule that the diff leaves out is a finding.

<!-- botica:covered -->
- The hook `botica` of `prek.toml`, run by the job `lint, type-check & test` of `tests.yml`:
  `no-tautological-assert` and `test-no-subprocess`.
- The hook `botica-project` of `prek.toml`, run by the same job: the settings of pytest that
  `pytest-strict` and `test-no-network` require.
- The job `judgment rules` of `abide.yml`: `test-in-memory-adapters` on the Python tests and
  `test-through-interface` on the Python, TypeScript and JavaScript tests.
- The test `conformance.rs` of `botica-cli`, run by the job `fmt, clippy & test` of `ci.yml`: the
  findings of each rule on its fixtures against its `expected.json`.
<!-- /botica:covered -->

## Evidence

Each finding names the behaviour and the test, with file and line, and what in the test or the diff
shows that a wrong behaviour would pass. A deleted or loosened test names the commit of the diff
that changes it.
