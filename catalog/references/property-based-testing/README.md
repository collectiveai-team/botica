A property-based test states what holds for every input of a kind, and Hypothesis generates the
inputs. When one fails, Hypothesis shrinks it to the smallest input that still fails and prints it.

## What to read

- `test_the_merge_has_every_item_of_both_lists_in_order` compares `merge_sorted` with `sorted` for
  any two sorted lists. A test with chosen examples, such as `merge_sorted([1, 3], [2])`, passes on
  an implementation that indexes both lists without checking their length; this property fails on it
  and reports `left=[], right=[]`.
- `strategies.py` builds the models of the domain. `st.builds(InvoiceLine)` alone does not read the
  constraints of a pydantic `Field` (`min_length`, `gt`, `decimal_places`) and builds lines that the
  model rejects, so each field gets a strategy with the same bounds. The strategies live beside the
  tests, as the fakes do (`tests/fakes/`).
- `test_invoices.py` has two kinds of property that most code has: a round trip (what is written
  and read back is equal) and a relation between two calls (the total of two invoices joined is the
  sum of their totals).
- No test sets `@settings`. When the environment has `CI`, as GitHub Actions has, Hypothesis loads
  its `ci` profile: no deadline, no database, the same examples on every run, and the code of
  `@reproduce_failure` printed with a failure. Outside CI it keeps each failing example in
  `.hypothesis/` and replays it first on the next run; since 6.152.0 it writes a `.gitignore` in
  that folder. `pytest-randomly` does not seed Hypothesis: `--randomly-seed` reorders the tests but
  does not repeat the examples. `--hypothesis-seed=<n>` repeats them.

## A refactor checked against the old implementation

A refactor keeps the old implementation as an oracle, and a property asserts that the old and the
new one give the same outcome for every generated input. The oracle is a copy of the old code in
`tests/`, `legacy_batches.py`, and it is deleted with the test once the refactor has shipped.

- `outcome` calls a function and returns `("ok", value)`, or `("raised", type(exc))` when the call
  raises. `test_the_new_batches_give_the_outcome_of_the_old_for_every_input` compares the outcomes
  of `batches` and `legacy_batches`, so an input that the old code rejects is compared too.
- A plain `assert batches(items, size) == legacy_batches(items, size)` fails on every input where
  the old code raises, so a test written that way draws only the inputs that the old code accepts
  (`st.integers(min_value=1)` for `size`). With the check of `size` removed from `batches`, that
  test passes; the test of `outcome` fails and reports `items=[], size=-1`: the old code raised
  `ValueError` and the new one returned `[]`. A new version that raises where the old one returned,
  or that raises another type of exception, fails the same way.
- Hypothesis shrinks a failing input toward `[]`, `0` and `""`, so the failure it reports is at the
  empty and boundary values, as `items=[]` above.
- A test that passes shows that no generated input made the two differ. It does not prove that the
  two are equivalent for every input.

## What to adapt

- `merge_sorted`, `Invoice` and the properties are one example. Write a property for a function
  whose docstring states a constraint on values ("sorted", "not empty", "the total of the lines"),
  and keep the tests with chosen examples that document a case.
- Give each field of a model the bounds that the model declares; when a model changes its
  constraints, its strategy changes with it.
- A test of code that reads the clock, the network or a thread fails at random under Hypothesis as
  it would with other inputs, and sooner. Give that code a fake (`in-memory-adapters`) before
  writing a property for it.
- For more examples in CI, register the `ci` profile again with `settings.get_profile("ci")` as its
  parent and a larger `max_examples`, in `tests/conftest.py`.
