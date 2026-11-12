---
title: CODEOWNERS export
---

# CODEOWNERS export

`botica review export --format codeowners` writes a CODEOWNERS file from the entries of
`[[review.areas]]` ([Configuration](configuration.md#review)): each glob of an entry on one line
with the owners of the entry. The design is decision 4 of
[ADR 0018](https://github.com/collectiveai-team/botica/blob/main/docs/adr/0018-risk-tiers-and-classifier.md).

```bash
botica review export --format codeowners                  # .github/CODEOWNERS
botica review export --format codeowners --host gitlab    # .gitlab/CODEOWNERS, one section per area
botica review export --format codeowners --output -       # print it
botica review export --format codeowners --check          # fail when the file differs
```

| Option | Meaning |
|---|---|
| `--format codeowners` | The only format. |
| `--host github\|gitlab` | The code host that reads the file. Default `github`. |
| `--output <file>\|-` | The file to write, or `-` for the standard output. Default: `.github/CODEOWNERS` or `.gitlab/CODEOWNERS` under the root of the project, whatever the working directory. |
| `--check` | Write nothing, and exit with `2` when the file is missing or differs from what the export would write. It needs a file, not `-`. |

The command exits with `0` when it wrote or checked the file, and with `2` when it could not. It
replaces the whole file, including lines written by hand, and refuses to write through a symbolic
link.

## The order of the lines

GitHub and GitLab give a path the owners of the last line that matches it. The export sorts the
lines from the broadest to the narrowest, and the order of the entries in the configuration does
not change the file:

1. by the tier of the entry, from `low` to `high`, so a path that a `critical` entry and a `core`
   entry both select belongs to the owners of `critical`;
2. for the same tier, by the number of names that the glob spells out with no wildcard, whatever
   the area: `/src/` comes before `/src/auth/`, and the `/` at the end of a folder adds none;
3. by the text of the glob, and a file before the folder of the same name.

Two entries of one tier that give the same line with other owners are refused (see below), so the
order of the configuration never decides.

The paths that are always `platform` (the list in [Configuration](configuration.md#review)) come
after every entry, with the owners of all the `platform` entries. A `platform` or `critical` entry
with a higher tier does not take them. The export adds `/CODEOWNERS` and `/docs/CODEOWNERS` to that
list, since GitLab reads them before `.gitlab/CODEOWNERS`.

An entry with no `owners` (`core` or `periphery`) writes no line on either host, so its paths keep
the owners of a broader line before it, if there is one. On GitHub a line with no owner would take
its paths from those owners.

Two globs of one tier that overlap, as `**/Dockerfile` in `platform` and `services/**` in `core`,
are ordered by the names they spell out and then by their text, not by area: here
`/services/` comes later, and its owners take `services/api/Dockerfile`. To give such a path to
the owners of `platform` or `critical`, write that entry with a higher `tier`, or spell out the
path.

## How a glob is written

The configuration reads a glob as a `.gitignore` does. The export writes each glob so that
GitHub and GitLab select the same paths with it:

| In the configuration | Written | Selects |
|---|---|---|
| `docs/**`, `docs/**/*`, `/docs/` | `/docs/` | everything under `docs` |
| `README.md` (no `/`) | `**/README.md` | a file or folder of that name at any depth |
| `docs/v1/index.md` | `/docs/v1/index.md` | that path from the root |
| `src/**/auth/**` | `/src/**/auth/` | everything under each `auth` folder below `src` |
| `{a,b}/**` | `/a/` and `/b/` | one line per alternative |
| `{q,x/a}` | `/q` and `/x/a` | the `/` of one alternative anchors them all, as in a `.gitignore` |
| `**` | `*` | every path |
| `x/**/` | `/x/**/*/` | the folders below `x` and what they hold, and not the files directly in `x` |

A last name that is not followed by `/`, as `db/migr*` or `src/auth`, can name a file or a folder.
GitHub gives a folder all its contents, and GitLab gives it none, so on GitLab the export writes a
second line with the `/` of a folder: `/db/migr*` and `/db/migr*/`. On GitHub a last name `*`
gets that second line too, since GitHub reads `/g/*` as the files directly in `g`.

## GitHub

The file is one list. GitHub has no count of approvals in CODEOWNERS and no optional line;
`approvals` waits for the check of ADR 0018 decision 4. A `periphery` entry with `approvals = 0`
still writes its lines, so with the code owners review required, its paths need the approval of
its owners on GitHub.

## GitLab

The file has one section for each area that has lines, from `periphery` to `critical`. A section
asks for the largest `approvals` of the entries of its area, as in `[critical][2]`, and an area
whose entries ask for 0 is optional, `^[periphery]`. The paths that are always `platform` close
the section of `platform`. Since each area has its own section, two entries of different areas can
give the same line with other owners.

GitLab reads `CODEOWNERS` at the root, then `docs/CODEOWNERS`, then `.gitlab/CODEOWNERS`, and
takes the first one it finds. Without `--output`, the export refuses to write `.gitlab/CODEOWNERS`
while one of the first two exists.

## What the export refuses

The export fails and writes nothing when:

- the configuration has no `[[review.areas]]`;
- the configuration is not at the root of the git repository and there is no `--output`: GitHub
  and GitLab read CODEOWNERS only at the root, and its paths start there;
- a glob has a character class (`[...]`), an escape (`\`), a space, a `#`, `***` or a character
  that is not ASCII, which GitHub and GitLab do not read as the configuration does;
- a glob has an empty alternative, as `{src,}`, or more groups of braces than the configuration
  follows;
- an owner is a group of GitLab, `@org/group/team`, and the host is `github`;
- two entries of the same tier give the same line with other owners, as `src/**` in `core` and
  `/src/` in another `core` entry (on GitLab, in the same area): which one wins would depend on the
  order of the configuration.

Two globs of one tier that overlap without giving the same line, as `src/**` and `src/auth/**`,
are not refused: the one that spells out more names comes later and its owners win.

## In CI

`--check` fits a step of CI or a hook of prek, as `botica rules export --format agents-md --check`
does for `AGENTS.md`:

```bash
botica review export --format codeowners --check
```
