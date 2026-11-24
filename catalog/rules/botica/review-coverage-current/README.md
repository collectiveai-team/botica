## Directive

The `<!-- botica:covered -->` block of each review role matches the hook and job files that the
role declares under `covered` in its front matter. botica computes the block as `botica review
check` does, from the files of the working tree, and reports the `REVIEW.md` of each role whose
block differs.

```markdown
---
role: tests
covered:
  jobs:
    - file: .github/workflows/ci.yml
      id: test
---

<!-- botica:covered -->  reported here, after an edit of ci.yml
- job "test" in ".github/workflows/ci.yml": blake3 6d81f84d…
<!-- /botica:covered -->
```

The roles are the ones that `roles` in `[review]` of the configuration lists, the seven roles
when it lists none. Their folder is `dir` in `[review]`, else `.gitlab/review` when only it exists,
else `.github/review`. When both exist, botica reports it at the root of the project, as `botica
review check` refuses them, and reads no role. A role of that list with no `REVIEW.md` is reported
at its path. A line binds the blake3 digest of the whole source file, so any edit of `prek.toml` or of a
workflow makes the blocks that cover it stale. botica also accepts a source checked out with CRLF
line ends, since git stores it with LF.

A role that declares no `rules` under `covered` has a block of hook and job lines only, and botica
compares the whole block. A role that declares rules has rule lines too, whose digest binds the
configuration, the catalog and the version of botica; botica compares only its hook and job lines,
and `botica review check` still checks the rule lines.

botica also reports, at the file, what `botica review check` reports as unverified coverage: a role
with no `covered`, a source file that is missing, does not parse, or has a symbolic link in its
path (botica reads no file outside the project), a hook or a
job that its file no longer has, a duplicate declaration, and markers that are missing, duplicate
or out of order.

botica reads the roles and the sources from the root of the project, the folder of the
configuration. `botica review check` reads them from the root of the git repository; in a project
whose configuration sits below that root, the two read different files.

## Why

`botica review plan` and `botica review check` stop with `stale-coverage` when a block differs from
the files it covers. A change to `ci.yml` or `prek.toml` that leaves the blocks as they were passes
its own pull request and blocks every pull request after it merges. The rule reports the block in
the commit that changes the file.

## Suppression

The finding is about the coverage of the review roles, so only the configuration silences it:
`ignore = ["review-coverage-current"]`, or the path in `extend-ignores` of
`[rules.review-coverage-current]`. To fix it, replace the lines of the block with the ones the
message shows.
