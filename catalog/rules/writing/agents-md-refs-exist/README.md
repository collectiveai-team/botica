## Directive

Each `@<path>` reference of an `AGENTS.md` or a `CLAUDE.md` names a file or a folder that exists
in the repository. The rule runs once for the whole project, on every `botica lint`, over each
`AGENTS.md` and `CLAUDE.md` that the walk reaches, and reports the reference. An error by default.

A reference is an `@` at the start of a line, or after a space or one of `` ` ( [ " ' < ``,
followed by a path. The path has a `/`, or it ends in the extension of a text or source file
(`.md`, `.toml`, `.py`…). The rule leaves out:

- the fenced blocks of code (```` ``` ```` and `~~~`);
- an `@` after a letter or a `/`, as in an email or in `https://x.com/@user`;
- `@latest`, `@user` and `@pytest.fixture`, which have no `/` and no such extension;
- a path that a `@` follows, as in `@scope/pkg@1.0`;
- a path with a `/`, no such extension and a first folder that the project does not have, as in
  the npm package `@types/node`;
- a path that starts with `/` or `~`, outside the repository;
- a missing path that `.gitignore` leaves out: a tool writes it, as `npx skills` writes
  `.agents/skills/`, and a fresh clone lacks it.

A reference resolves from the folder of its file, as Claude Code reads it, or from the root of the
project. The rule looks for the path on disk.

## Why

An agent reads the file that a reference names and follows its instructions. When the file is
missing, the agent does not stop: it goes on without those instructions. A file that is renamed or
moved leaves its references behind, and no reader of the `AGENTS.md` sees that it broke.

## Do

```markdown
Text follows the style guide (`@docs/style.md`).
See @docs/agents/domain.md and install with `npm i @types/node`.
```

## Don't

```markdown
Text follows the style guide (`@docs/styles.md`).
```

## Suppression

The rule is of project scope, and only the configuration of botica silences it: the file in
`extend-ignores` of `[rules.agents-md-refs-exist]`, or `ignore = ["agents-md-refs-exist"]`. A
comment that names it is `invalid-suppression`.
