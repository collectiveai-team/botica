# Domain Docs

How the engineering skills read the domain documentation of this repo when they explore the code.

Layout: single-context, one `GLOSSARY.md` and `docs/adr/` at the repo root.

## What the skills read before exploring

- `GLOSSARY.md` at the repo root, or
- `GLOSSARY-MAP.md` at the repo root if it exists; it points at one `GLOSSARY.md` per context, and each one that the topic touches is read.
- `docs/adr/`: the ADRs that touch the area of the task. A multi-context repo also has `src/<context>/docs/adr/` for the decisions of one context.

A missing file is skipped without a mention, and nothing suggests creating it. The `/domain-modeling` skill (reached through `/grill-with-docs` and `/improve-codebase-architecture`) creates these files when terms or decisions are resolved.

## File structure

Single-context repo:

```
/
├── GLOSSARY.md
├── docs/adr/
│   ├── 0001-event-sourced-orders.md
│   └── 0002-postgres-for-write-model.md
└── src/
```

Multi-context repo (`GLOSSARY-MAP.md` at the root):

```
/
├── GLOSSARY-MAP.md
├── docs/adr/                          ← system-wide decisions
└── src/
    ├── ordering/
    │   ├── GLOSSARY.md
    │   └── docs/adr/                  ← context-specific decisions
    └── billing/
        ├── GLOSSARY.md
        └── docs/adr/
```

## The vocabulary of the glossary

An output that names a domain concept (in an issue title, a refactor proposal, a hypothesis, a test name) uses the term of `GLOSSARY.md`, and none of the synonyms that the glossary lists to avoid.

A concept that is not in the glossary is either a new term the project does not use or a gap to note for `/domain-modeling`.

## A conflict with an ADR

An output that contradicts an existing ADR names it:

> _Contradicts ADR-0007 (event-sourced orders); worth reopening because…_
