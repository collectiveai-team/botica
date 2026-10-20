---
name: botica-rule-author
description: Write a new botica lint rule, either in the built-in catalog (catalog/rules) or in a catalog of a project (.botica/rules/<name>), with its fixtures, and check that it loads and conforms. Use when the user asks to add, write, draft or port a botica rule, a pattern rule, an extension rule or a judgment rule.
---

# Botica rule author

A rule of botica is a folder `<family>/<slug>/` that `botica rules new` writes and the catalog
loads. This skill writes the rule through that command, fills in the files it leaves with
`TODO`, and checks the result with `botica rules conform`. A model helps write the rule once;
`botica lint` never calls one.

## Before writing

1. Run `botica rules --all` and `botica explain <slug>` for the rules that look close. An
   existing rule may need an option, not a new rule.
2. Pick the kind of the rule:
   - the pattern kind, the first value of `--kind` in `botica rules new --help`: a YAML pattern
     over the syntax tree of one file. The first choice.
   - `extension`: logic that YAML cannot express. A catalog of a project only, never the
     built-in catalog. `--runtime python`, `node` or `command`.
   - `external`: another tool enforces the standard through its configuration.
   - `judgment`: no deterministic check can express it, with or without configuration from the
     repository. The `why-not-deterministic` field must say why (ADR 0005). Look for a
     deterministic rule with a required option first.
3. Pick the catalog. `catalog/rules` is the built-in one. A project writes into
   `.botica/rules/<name>`, the folder with a `catalog.toml`, and the family must already exist
   there (`<family>/family.toml`); `rules new` does not create it.

## Write

```bash
botica rules new structure/no-foo --kind <pattern kind> --language python --standard CES-63
botica rules new structure/no-blobs --kind extension --runtime python --language python \
  --catalog .botica/rules/acme
```

The command writes `rule.toml` (preview, off by default, the next code of the family), a README
with the sections of its template, the implementation and empty `fixtures/`. Then:

1. Replace each `TODO` in `rule.toml` and the README. The sections of a README are a closed
   list in a fixed order, and the catalog rejects a README that does not match its kind.
2. Put source files in `fixtures/valid/` (no finding) and `fixtures/invalid/` (one finding each).
3. Write `fixtures/expected.json` with the range of each finding.
4. For a pattern rule, write the YAML (`python.yml`, `typescript.yml`). In a project catalog
   its `id` is the full id, `<catalog>/<slug>`.
5. For an `extension` rule, write `check` in the script. A finding is a start and an end in
   bytes, and a message. Declare the dependencies of a Python script in its PEP 723 block.

## Check

```bash
botica rules conform catalog/rules              # or .botica/rules/<name>
botica rules --all                           # the rule is listed, with its code
botica lint --select <slug> --preview <path> # preview rules run only with --preview
```

`conform` runs each rule over its fixtures, with and without each optimisation, and compares the
findings with `expected.json`. It exits 1 when a rule fails. A rule that fails the fixtures is
not finished; change the rule or the fixtures with a reason, not the expectation to match the
output.

A judgment rule is calibrated with `botica rules test --judge abide <slug>`, which sends its
fixtures to abide and needs abide and its key. If abide is not available, say that the rule is
uncalibrated. abide reads `true` of a boolean question as the violation: the question asks whether
the file breaks the rule, and `criteria.true` describes the broken case. A question asked the
other way round fails every fixture of the rule, with each band reversed.

## Finish

- Start the rule at `status = "preview"` and `default-enabled = false`. Promoting it is a
  separate decision.
- Messages state the fix of their own rule, name no other rule and carry no CES code (the output
  takes the standard from `rule.toml`).
- A change to the built-in catalog touches one standard: `scripts/check_rule_change.py` fails a
  change that adds a rule and touches more than that rule needs.
- Text in the README states what the rule does, in plain sentences.
