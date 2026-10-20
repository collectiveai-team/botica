# Engineering Standards — CES (Collective Engineering Standard)

How the standards of the house are numbered, cited and connected to the rules of botica.

## What a CES is

A standard is the file `standards/CES/<number>-<slug>.md`, which holds its text (ADR 0004). A rule
of the catalog cites it in the `standard` field of its `rule.toml`. A standard has any number of
rules, none included.

## Two identifiers, two jobs

| Layer | Value | Used in |
|---|---|---|
| Catalog code | `CES-<number>` | commit and pull request references, human citation |
| Machine id | kebab-case slug (`log-no-print`, `api-schemas-extra-forbid`) | botica rule id, prek hook id, `# botica: ignore[<slug>]` |

The CES number is not part of a machine id: tooling and suppressions key on the slug.

## Invariants

- A rule cites a CES only if `standards/CES/<number>-<slug>.md` exists.
- One CES is one standard, and it may have several rules: `CES-79` has the four `no-dict-*` rules.
- The number is the next free one of the set, `max + 1`. The numbers up to `CES-119` are those
  of the issues where the standards were first proposed and are kept; from `CES-120` on, `CES-N`
  is no longer the issue `#N`. A number is never reused or changed.
  `scripts/check_rule_change.py` fails a change that leaves two standards of a set with the same
  number.

## Violation messages carry no code

```yaml
message: "Libraries log, they don't print — use the house get_logger."
```

The text output names the rule of each finding (`warning [log-no-print] …`), the JSON format
carries the `standard` of the rule in each diagnostic and SARIF in the `properties` of each rule,
and `botica explain <slug>` prints it. The slug remains the suppression key
(`# botica: ignore[<slug>]`).

A message states its own rule and the fix. It names no other rule and gives no other rule's
suppression key. What a standard's rules share is written once, in the file of the standard or in
the README of each rule.

## Where the pieces live

- `AGENTS.md` lists no standard. Its `## Botica rules` section holds the block that
  `botica rules export --format agents-md` writes: one line for each active judgment rule.
- `standards/<set>/<number>-<slug>.md`: the text of a standard, written and kept by people, with
  its rationale, examples and migration. It may repeat what the README of a rule in
  `catalog/rules` says, which is the documentation of one implementation.
- The topics of `references/` (`catalog/references/` for botica's own) hold code to read and copy,
  read with `botica refs show <topic, rule or standard>`.
