# Scoped unignore op: bounded destructive editing in a clean-adds-only engine

**Status:** accepted (CES-107)

The engine is clean-adds-only — it must never edit, merge, or overwrite an existing target file.
That invariant is what makes it safe to point at someone else's repo, and CES-30 depends on it.
CES-107 makes `skills-lock.json` a tracked manifest, which breaks on any repo that already
gitignores it: the scaffolder would restore skills happily while the manifest stayed untracked
forever, and would report success. Un-ignoring requires deleting a line from a file the repo
already owns.

We added an `unignore` op that removes **one literal line drawn from an enumerated whitelist**
(`UNIGNORE_WHITELIST` in `scaffolding/skills.py`) from `.gitignore`, gated behind a `Decision`
that defaults to yes. It refuses any line outside the whitelist by raising.

Two properties keep the op honest, both learned the hard way:

- **Detection matches remediation.** The op is planned only when `.gitignore` carries the literal
  line. `git check-ignore` is satisfied by `*.json`, by `/skills-lock.json`, by
  `.git/info/exclude` — planning off that broader answer produces a run that removes nothing and
  reports success. Anything we cannot fix with a literal line removal is a warning instead.
- **The rewrite is byte-preserving.** Newline translation is disabled on both ends, so a CRLF or
  mixed-ending `.gitignore` keeps its endings and a file without a trailing newline does not gain
  one. Removing one line must not produce an all-lines diff.

## Considered options

- **Check and instruct instead.** `scaffolding check` fails with the exact remedy and a human
  makes the one-line edit. Preserves the invariant perfectly. Rejected because the same run that
  detects the problem can fix it, and every legacy repo would otherwise carry a manual step.
- **General `edit` / `remove-line` capability.** Rejected: clean-adds-only would stop being a
  property of the engine and become a convention nobody can verify.
- **Do nothing; grandfather affected repos.** Rejected: the repos that most need the standard are
  exactly the ones that already ignore the file.

## Consequences

"Clean-adds-only" is now precisely "clean-adds, plus removal of literal lines on an explicit
whitelist." This remains checkable because the whitelist is a literal frozenset in code, not a
pattern language — you can read it and enumerate every destructive edit the engine can perform.

That claim is only worth making if it is true in the report as well as in the code, so the op is
reported under `edits` in `plan --json`, not under `clean_adds`. A field named `clean_adds` that
contains a line deletion falsifies the invariant it is supposed to document.

`AGENTS.md` states the amended invariant, not the absolute one. The ADR is not always in context
and `AGENTS.md` is; leaving the two in contradiction would mean the next agent reads
"never edits existing files", finds this op, and deletes it as a violation.

Future components will cite this precedent. The line to hold is the *literal whitelist*, not the
op: adding a new literal is a reviewable one-line change, whereas accepting a regex, a glob, or a
caller-supplied string would give the capability away entirely.

Note what this does **not** authorise. `skills-lock.json` is written only by the third-party
`skills` CLI, as a side effect of a run op; the engine never writes it. An earlier draft of
CES-107 had scaffolding reconstruct that file when it could not be parsed, which is a destructive
write to an existing target outside the whitelist — exactly what this ADR exists to bound. That
path was removed: an unparseable manifest is now deferred with a warning.
