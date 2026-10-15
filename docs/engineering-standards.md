# Engineering Standards — CES (Collective Engineering Standard)

Single source of truth for how house engineering rules are coded, cited, and shipped.
`AGENTS.md` and `README.md` reference this file rather than restating it.

## What a CES is

A **CES — Collective Engineering Standard** is one house rule that the scaffolder ships into
target repos (as a botica rule, a prek hook, a `pyproject` setting, or an AGENTS.md
directive + detail file). Each standard is the file `standards/CES/<number>-<slug>.md`, which holds
its text (ADR 0004).

## Two identifiers, two jobs

| Layer | Value | Used in |
|---|---|---|
| **Catalog code** | `CES-<number>` | AGENTS.md index, commit/PR refs, human citation, violation messages |
| **Machine id** | kebab-case **slug** (`log-no-print`, `api-schemas-extra-forbid`) | botica rule id, prek hook id, `# botica: ignore[<slug>]` |

Always written together where space allows: **`CES-46 · log-no-print`**.

Never encode the CES number in the machine id — slugs are what tooling and suppressions key
on, so they must stay stable and semantic.

## Invariants

- **Every CES is a file.** A rule cites a CES only if `standards/CES/<number>-<slug>.md` exists.
- **One CES = one standard.** A standard may ship **multiple slugs** — e.g. `CES-79` ships the
  four `no-dict-*` botica rules; an architecture rule may ship a botica rule *and* an
  import-linter contract under the same CES.
- **The number is the next free one of the set**, `max + 1`. The numbers up to `CES-119` are those
  of the issues where the standards were first proposed and are kept; from `CES-120` on, `CES-N`
  is no longer the issue `#N`. A number is never reused or changed.
  `scripts/check_rule_change.py` fails a change that leaves two standards of a set with the same
  number.

## Violation messages embed the code

```yaml
message: "CES-46 (log-no-print): libraries log, they don't print — use the house get_logger."
```

The slug remains the suppression key (`# botica: ignore[<slug>]`), so embedding the code in
the message text never affects tooling stability.

### Messages are self-contained (no cross-rule references)

A violation message states **its own** rule, the fix, and its CES code — nothing else. It must
**not**:

- name a sibling rule (e.g. "hidden from the no-dict-return rules"), or
- prescribe another rule's suppression key (a message for `no-dict-alias` telling you to add
  `# botica: ignore[no-dict-return-annotation]` is wrong — and wouldn't even silence itself).

Scattering policy across sibling messages is context-poisoning: one rule ends up restating or
superseding another's decision. The grouped rationale, the relationships between a standard's
slugs, and the escape-hatch mechanism live **once**, in the file of the standard
(`standards/<set>/<number>-<slug>.md`; for a rule that `botica lint` measures, the README of its
folder in `lint/rules`) — the single source of truth. If a message needs more than "what +
fix + code", that belongs in the file of the standard, not the message.

## Where the pieces live

- **Always-on index:** a marker-delimited `## Engineering Standards` section in `AGENTS.md`
  listing every rule that `botica lint` does not measure (terse directive + `[prek]`/`[judgment]`
  marker + an `@standards/<set>/<number>-<slug>.md` pointer), and one sentence that sends the
  reader to `uvx botica rules` and `uvx botica explain <slug>` for the rest.
- **On-demand detail:** `standards/<set>/<number>-<slug>.md` — full rationale, examples,
  migration. A rule that `botica lint` measures has its own documentation, the README of its
  catalog folder, and its `standard` field names the standard it implements.
- **Canonical code:** `.agents/snippets/` — drop-ins for new repos, comparison templates for existing
  ones.

(Rationale for index-in-AGENTS.md over packaged skills: passive always-on context outperforms
on-demand retrieval for horizontal standards — see PRD #78.)

## References

- Roll-out plan: **PRD #78** (`meta/standards-integration`).
- The text of each standard: `standards/CES/`.
- The rules that `botica lint` measures: the catalog (`lint/rules`) lists them, each with the
  standard it implements, and documents them.
