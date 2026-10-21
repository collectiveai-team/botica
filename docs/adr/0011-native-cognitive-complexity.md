# botica measures cognitive complexity

**Status:** proposed

`cognitive-complexity` (BTSTRUC006, CES-110) is an external rule: the hook `complexipy` of
`prek.toml` and of `catalog/recipes/sections/prek-python-tools.toml` measures it, and botica only
checks that the hook is there (`external-config`). The rule covers Python alone, `botica lint` does
not report its findings, `--diff` and `--staged` (ADR 0009) do not filter them, and a repository
with old findings takes a snapshot of complexipy that botica does not read. complexipy 8.1.0 is
written in Rust, but its crate `complexipy-core` is not published on crates.io, and the parser it
uses (`ruff_python_parser`) is not either.

## Decision

1. **A crate, `botica-complexity`.** It takes a language and a source text and returns each function
   it measures with its qualified name, its range and its cognitive complexity. It parses with the
   tree-sitter grammars of `ast-grep-language`, which the workspace already builds. `botica-core`
   depends on it and registers the native rule `cognitive-complexity` in `NATIVES`; the crate knows
   nothing of rules, findings or configuration.
2. **Four languages: Python, TypeScript, Go and Rust.** The rule keeps its slug and its code, its
   implementation becomes `kind = "native"` with `languages = ["python", "typescript", "go",
   "rust"]`, `scope = "file"` and `needs = "syntax"`, and it stays preview and off by default.
3. **What is measured.** A function is measured on its own when it is one of these, and every
   function, lambda, closure or arrow function inside it adds to it, one level of nesting deeper, as
   complexipy does:
   - Python: a `def` at the top of a module, and a method of a class at the top of a module.
   - TypeScript: a `function` at the top of a module, a function or arrow function assigned to a
     `const` or `let` at the top of a module, and a method of a class at the top of a module.
   - Go: a `func` and a method.
   - Rust: a `fn` in a module, inline modules included, and a `fn` of an `impl` or a `trait`.

   The qualified name joins the names with `::`: `Order::total` for a method, `tests::parses` for a
   function of an inline module of Rust. Code outside these functions is not measured.
4. **How it counts.** The counts follow the paper of SonarSource that CES-110 links. Each `if`,
   ternary, `switch` or `match`, loop and `catch` or `except` adds one plus its nesting. Each `else`,
   `elif` or `else if`, each sequence of the same boolean operator (`and`, `or`, `&&`, `||`, `??`),
   each `break` or `continue` with a label, each `goto` and each direct recursive call adds one.
   Python follows `crates/complexipy-core/src/cognitive_complexity.rs` of complexipy 8.1.0
   where the paper leaves a choice open, such as comprehensions and the arms of a `match`.
5. **Parity with complexipy.** The 24 files of `tests/src` of complexipy 8.1.0 and the values that
   its `tests/main.py` expects become fixtures of the crate, with the MIT license of complexipy next
   to them, and the README of the crate credits complexipy. A value that differs fails the test. A
   difference kept on purpose is listed in that README with its reason, and its test expects the
   value of botica. Moving to a later release of complexipy is a change of its own. TypeScript, Go
   and Rust have no such reference, and their fixtures follow the examples of the paper.
6. **The finding.** Its range goes from the line of the name of the function (`def`, `fn`, `func`,
   `function`, or the name of a method) to the end of its body, so `--diff` keeps it when a change
   touches any line of the body. The message names the function, its value and the limit:
   ``Order::total has a cognitive complexity of 23 (the limit is 15)``. The option `max` (int,
   default 15, at least 1) sets the limit, and a function above it is reported.
7. **Silencing a function.** `# botica: ignore[cognitive-complexity]` or `# noqa: BTSTRUC006` on the
   first line of the range, or `// botica: ignore[cognitive-complexity]` in the other three
   languages. botica does not read `# complexipy: ignore` or `# noqa: complexipy`.
8. **A baseline of known findings.** `[tool.botica]` takes `baseline = "<path>"`, a TOML file kept in
   the repository, with one entry per known finding: the rule, the path, the qualified name and the
   value. `botica lint --write-baseline` writes the file from the findings of the run and exits 0. A
   finding whose rule, path and name match an entry is dropped while its value is at most the one
   of the entry; above it, it is reported, and the message gives both values. An entry that matches
   no finding is dropped at the next `--write-baseline`. Only a finding that carries a qualified
   name and a value can enter the baseline, and `cognitive-complexity` is the only rule that gives
   them for now. The baseline applies after the comments of decision 7 and before the filter of
   `--diff`.
9. **The hook of complexipy leaves.** `config/prek.toml` of the rule and the section `# MARK:
   complexipy` of `prek-python-tools.toml` are removed, and the recipe `base-python` selects
   `cognitive-complexity`. This repository keeps its hook `complexipy` in `prek.toml` until a
   release of botica ships the rule, since the hooks of the mirror run 0.13.0; then its `select`
   lists the rule and the hook goes.

## Considered options

- **Python only, as complexipy.** Rejected: TypeScript, Go and Rust code would go unmeasured, and
  the crate would need a second design for them later.
- **Depend on `complexipy-core`.** Rejected: it is not published on crates.io, it covers Python
  only, and it would bring `ruff_python_parser`, which is not published either.
- **A module of `botica-core`, as `coverage.rs` and `commit.rs`.** Rejected: the measure has no use
  for findings or configuration, and a crate of its own keeps its fixtures and its parity tests
  apart.
- **The range on the name of the function alone.** Rejected: with `--diff`, a change inside the body
  of a complex function would not be reported.
- **A baseline keyed by the range.** Rejected: a range moves when the code above it changes, as ADR
  0009 notes.
- **No baseline, `--diff` alone.** Rejected: a run without a diff, such as `botica lint` on a whole
  repository, would still report every old finding.
- **Read the snapshot of complexipy.** Rejected: it covers Python only and it ties the baseline to
  the format of another tool.

## Consequences

- `botica rules`, `botica explain` and the editor show the findings of `cognitive-complexity`, and
  `--diff` and `--staged` filter them.
- A project that selects the rule and has TypeScript, Go or Rust code gets findings it did not get
  from complexipy; `--write-baseline`, or `extend-ignores` of `[rules.cognitive-complexity]`,
  takes them.
- A project that installed the hook of complexipy keeps it until it removes it; `external-config`
  no longer reports its absence. The fixtures of `external-config` use another external rule.
- A function with `# complexipy: ignore` is reported until the comment is replaced by the one of
  decision 7.
- The README of the rule describes the native rule, with the limit, the baseline and the comment
  that silences a function. CES-110 keeps its directive and its reasons, names the four languages
  and no longer names the hook of complexipy, its snapshot or its sections on the threshold,
  migration and suppression, which the README of the rule holds.
