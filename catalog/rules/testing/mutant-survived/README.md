## Directive

A test fails on each mutant that cargo-mutants makes on the lines a change adds. cargo-mutants
changes the code (`==` to `!=`, the body of a function to `Default::default()`) and runs the tests
on each change; a mutant that the tests pass on survives. `botica lint --mutants
mutants.out/outcomes.json` reads its report and reports each surviving mutant as a warning at its
line.

## Why

A line that the tests run and whose change no test notices is covered and untested: coverage counts
it, and a bug there passes the suite. A surviving mutant names that line and the change that went
unseen, so the fix is a test with an assertion on the behaviour it changes, or the deletion of code
that nothing needs.

## Exceptions

A mutant that leaves the behaviour as it was, such as a change to a log message or to a value
that the code computes twice, survives every test. Such a mutant is skipped in the configuration of
cargo-mutants, or silenced with a comment that gives the reason.

## Tool

[cargo-mutants](https://mutants.rs/) writes `mutants.out/outcomes.json`, with one outcome for each
mutant; botica reads the format of cargo-mutants 27.1.0 and reports the outcomes whose `summary` is
`MissedMutant`. A mutant that times out, or that does not build, is left out. With
`--in-diff`, cargo-mutants makes only the mutants on the lines of a diff, and with `--diff`,
`--staged` or `--diff-file`, botica keeps only the findings on the lines the change adds:

```bash
git diff origin/main... > git.diff
cargo mutants --in-diff git.diff
botica lint --mutants mutants.out/outcomes.json --diff-file git.diff
```

## Configuration

cargo-mutants reads `.cargo/mutants.toml`. A run that a CI job makes on the diff of a pull request
needs no configuration; a function that no mutant should touch is listed there:

```toml
exclude_re = ["impl Display", "replace .* -> String with String::new\\(\\)"]
```

## Do

```rust
pub fn is_even(n: u32) -> bool {
    n % 2 == 0
}

#[test]
fn three_is_odd() {
    assert!(!is_even(3));
}
```

## Don't

```rust
pub fn is_even(n: u32) -> bool {
    n % 2 == 0
}

#[test]
fn is_even_runs() {
    is_even(3);
}
```

## Suppression

A comment on the line of the mutant, with the reason after `--`:
`// botica: ignore[mutant-survived] -- the log message changes nothing a caller sees`.
`botica: ignore-file[mutant-survived] -- <reason>` silences the whole file, and
`#[mutants::skip]` on the function keeps cargo-mutants from making its mutants at all.
