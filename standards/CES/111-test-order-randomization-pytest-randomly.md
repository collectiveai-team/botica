---
id: CES-111
title: Test order randomization
---

## Directive

A test suite does not depend on the order in which its tests run.
[`pytest-randomly`](https://github.com/pytest-dev/pytest-randomly) is in the `dev` dependency
group and activates as a pytest plugin on every `pytest` run: local, prek and CI.

## Why

Tests that depend on order or leak state are a classic latent bug. They become more likely as
independent AI-agent sessions add tests to the same suite: each session sees only its own context
and cannot see the shared fixtures or module state that the tests of another session leave dirty.
`pytest-randomly` finds them.

## When a test fails under randomization

The output prints the seed (`Using --randomly-seed=1234`), and the failure reproduces with
`pytest -p randomly --randomly-seed=1234`. The fix is in the coupling underneath: a shared mutable
fixture, module-level state, or a test that needs the side effect of a previous one. The failure is
a real bug that the earlier fixed order hid.

## Migration (existing repos)

Adding the plugin can surface order dependencies that the suite already has, and each is a bug to
fix. As a last resort during triage, `-p no:randomly` goes on one specific CI job while the
coupling is fixed. It is written down in the job and it is never permanent.

## Suppression

A single test has no opt-out. `-p no:randomly` disables the plugin for a whole run, and it is a
temporary step during triage that points back to the coupling it hides.
