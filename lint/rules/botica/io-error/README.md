## Directive

A path given on the command line that does not exist, or a file given explicitly that cannot be
read, is an error reported at line 1, column 1. The rest of the paths are still checked.

## Why

Without it, `botica lint src/typo.py` exits 0 having checked nothing, and a CI job with a
misspelled path passes silently. Ruff does the same with `E902`, which it also calls `io-error`.

A file that disappears while the walk is running is ignored without a warning. Hooks never hit
this rule: prek drops deleted files before it calls the hook.

## Suppression

There is no file to hold a comment, so this finding cannot be suppressed in the source. Exclude it
in the configuration.
