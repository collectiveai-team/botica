## Directive

The cognitive complexity of a Python function is at most 15.

## Why

A nested function is hard to read.

## Tool

complexipy measures it, as a hook of prek.

## Configuration

The hook `complexipy` in `prek.toml`.

## Do

A function with few nested conditions.

## Don't

A function with many nested conditions.

## Suppression

The `exclude` pattern of the hook.
