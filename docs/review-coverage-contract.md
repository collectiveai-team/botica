# Review coverage declarations

ADR 0017 decision 6 promises generated `botica:covered` blocks. The original role documents
contained prose, without a generator or a role-to-check mapping. The coordinator's 2026-11-17
engineering default defines explicit destination-owned declarations as the source of truth.
This changes no risk tier, size gate or human-routing rule.

## Declaration

The optional `covered` table is YAML frontmatter in the role's trusted `REVIEW.md`:

```yaml
covered:
  rules: [file-size-guard]
  hooks:
    - file: prek.toml
      id: botica
  jobs:
    - file: .github/workflows/ci.yml
      id: test
```

The three lists default to empty. `covered: {}` explicitly declares no exclusions.
Omitting `covered` leaves legacy coverage unverified, including an empty legacy marker block.
Unknown frontmatter or declaration keys, duplicate declarations and invalid references fail
validation. Reference files are literal repository-relative paths, without traversal or symbolic
links. IDs and rule names are data, never commands.

Rule names must be canonical slugs, including the catalog prefix for project rules. The resolver
reads rule selection/configuration keys from the selected destination configuration, without
requiring unrelated historical settings to match this version's full schema. It loads the
embedded catalog plus destination-owned `rule-dirs` and `catalogs`, and requires the declared rule
to be active in configuration-level resolution. Catalog scripts are read as data or fingerprinted;
none runs.

Hook declarations require an exact `repos[*].hooks[*].id` in the named TOML or YAML source.
Job declarations require an exact key under a GitHub-style `jobs` mapping. For a source without
that mapping, a GitLab-style top-level job must be a mapping containing `script`, `trigger` or
`extends`. Includes, remote templates and dynamically assembled jobs are not resolved; a
declaration that cannot be verified is unverified, not current.

## Fingerprints and generated text

The renderer sorts references by kind, source file and ID. Each line contains the kind,
JSON-quoted ID and file, and a BLAKE3 fingerprint. `review plan` and `review check` report the
expected block; neither edits the destination or working files.

- Hook/job fingerprints hash the entire named file's raw bytes. An unrelated edit in that file
  can therefore make the block stale.
- Rule fingerprints hash the serialized object containing the resolved catalog export
  definition, selected configuration digest, botica version and the sorted destination project
  catalog source fingerprints. Catalog fixtures are excluded, as in ordinary catalog loading;
  other tracked catalog files, including command scripts, are bound even when not parsed.
- Role and shared-policy identities are separate raw-byte digests in the consumer report.

The block must have one balanced pair of markers. Line endings are normalized to LF for the
comparison; the role's identity digest still hashes its raw bytes. A missing or differing block
is stale. A missing source, inactive/unknown rule or absent declaration is unverified.
Both statuses make the operation incomplete and exit with 2.

Fingerprints establish configured presence and source freshness. They do not establish that a
check ran, passed, applies to every changed path/language or semantically covers every invariant
of the role. Local skill checks establish a nonempty regular `SKILL.md` and its digest, not its
origin or the truth of its instructions.

## Migration

Preserve existing prose outside a generated block when its declared references are unambiguous.
Do not infer a role-to-check mapping from filenames, tags, a role name or a model's judgment.
Unmapped prose stays legacy/unverified until owners declare its sources; copying prose into a
generated block does not make it current.

The repository's initial migration is bounded to the empty `refute` declaration and the two
workflow jobs explicitly named by `spec`. Other role blocks remain legacy/unverified.
Named tests, implicit compatibility checks and prose about semantic coverage have no declaration
format in this implementation.
