---
role: architecture
applies:
  paths: []
  tiers: [medium, high]
stages: [local, ci]
blocking: P1
max-findings: 8
---

## Invariants

- Dependencies between the modules and packages of the repository point the way the ADRs and the
  layers of `import-layers` set, with no cycle.
- An abstraction has more than one user, or a second one that the change or an ADR names.
- A concept has one name, the one of `GLOSSARY.md` or of the code that already holds it.

## Check always

- A new import or a new dependency of a manifest between packages of the repository, and its
  direction.
- A new type, trait, interface or module, and how many users it has.
- A new name for a concept that already has one.
- A function that orders the steps of a task and also carries one of them out.

## Do not report

- What `botica lint` reports under the rules of the family `architecture`, such as
  `import-layers`.
- A refactor that the diff does not need.

<!-- botica:covered -->
<!-- /botica:covered -->

## Evidence

Each finding names the modules and the import, the type or the name, with file and line, and the
ADR or the line of `GLOSSARY.md` it goes against.
