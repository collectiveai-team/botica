---
title: Catalogs of a project
---

# Catalogs of a project

A project adds rules in `.botica/rules/<catalog>/`, under the folder of `botica.toml`, and
commits them. A rule of a project runs beside the built-in rules, takes the same configuration and
the same comments, and is listed by `botica rules --all`.

## A first rule

A catalog starts with its `catalog.toml` and the `family.toml` of each family; `botica rules new`
writes the rest of the folder of a rule, with `TODO` where the rule needs its text:

```bash
mkdir -p .botica/rules/acme/structure
printf 'code = "ACME"\n' > .botica/rules/acme/catalog.toml
printf 'code = "STRUC"\ndescription = "The layout of the project"\nextends = "structure"\n' \
  > .botica/rules/acme/structure/family.toml
botica rules new structure/no-blobs --kind ast-grep --language python --catalog .botica/rules/acme
botica rules conform .botica/rules/acme      # the rule over its fixtures
botica lint --preview --select acme/no-blobs # a new rule is a preview rule, off by default
```

[Write a rule](rules.md) fills in the folder, and [Extensions](extensions.md) covers a rule that is
a program.

## Layout

A catalog has the layout of `catalog/rules/` with a `catalog.toml` on top:

```text
.botica/rules/acme/
  catalog.toml           # code = "ACME"
  structure/
    family.toml          # code = "STRUC", description = "…"
    no-blobs/
      rule.toml          # id = "acme/no-blobs", code = "ACMESTRUC001"
      README.md
      python.yml         # id: acme/no-blobs
      fixtures/
```

The id of a rule is `<catalog>/<slug>` and its code is the code of the catalog, the code of the
family and 3 digits. The code of a catalog is letters A-Z, does not start with `BT` and is not a
text prefix of the code of another catalog. The name of a catalog is not the name of a family of
the built-in catalog (`structure`, `logging`…): a catalog with such a name does not load. A catalog
takes rules of every kind but `native`. The `covered-by` of a judgment rule of a catalog names the
rules of that catalog by id (`acme/no-blobs`) and those of the built-in catalog by slug.

## Where the catalogs are

`rule-dirs` (`[".botica/rules"]` by default) lists the folders whose subfolders are catalogs, and
`[catalogs]` declares one catalog by the folder it is in, relative to the folder of `botica.toml`:

```toml
rule-dirs = [".botica/rules", "team-rules"]

[catalogs]
shared = { path = "../shared-rules" }   # the catalog `shared`; the folder holds its catalog.toml
```

The name of a catalog is the name of its folder in a `rule-dirs` folder, or its key in
`[catalogs]`, and it is used once. A folder of the project that holds catalogs is left out of the
walk, as `.botica` is. The rules of a declared catalog are read from its folder: an extension script
is found there.

## Families and replaced rules

A family of a project may have the name of a built-in family, as `structure` above. Every command
then prints a warning on stderr, unless its `family.toml` says that it extends the built-in family
on purpose:

```toml
code = "STRUC"
description = "…"
extends = "structure"   # a family of the built-in catalog
```

`extends` changes one thing: `botica rules export` puts the rules of the family in the folder and
the section of the built-in family, after its rules. The ids and the codes stay those of the
catalog. A project replaces a built-in rule with `replaces` in its `rule.toml` and `[replace]` in
`botica.toml`; `botica rules --all` shows which rule replaces which.

## Select, configure and silence

`acme/no-blobs` and `ACMESTRUC001` name a rule and `ACMESTRUC` a family, in `select`, `ignore`,
`--select` and `--ignore`; `botica explain` takes a rule. `[rules."acme/no-blobs"]` configures it,
and `# botica: ignore[acme/no-blobs]` or `# noqa: ACMESTRUC001` silences it. A catalog that does
not load stops the command with exit code 2 and names the file and the problem.
