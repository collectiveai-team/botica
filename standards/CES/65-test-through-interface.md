---
id: CES-65
title: Test through the interface
---

## Directive

Tests go through the public interface, the same seam that production code calls, and assert on
observable behaviour. A test does not reach into private attributes, call underscore-prefixed
methods or assert on internal state. What is not part of the contract is not tested directly.

## Why

A test bound to internals breaks on a refactor that keeps the behaviour, and a test that breaks on
every refactor gets deleted. A test through the interface survives the refactor and checks the
contract. Code that cannot be tested without reaching into its internals is missing a public query.
