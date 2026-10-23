# `external-config` reads the settings of a tool where the tool reads them

**Status:** accepted

`external-config` (`BT004`, ADR 0006) read one TOML file at the root of the project, the one that
the fragment of an external rule names, and compared the keys of the fragment with the keys of the
file. ruff reads the same settings from `.ruff.toml`, `ruff.toml` or the `[tool.ruff]` of
`pyproject.toml` (<https://docs.astral.sh/ruff/configuration/#config-file-discovery>), and a family
of rules is on whether it is under `select` or `extend-select`. A repository that kept its settings
in `ruff.toml`, or that added a family with `extend-select`, was reported for a configuration that
ruff applies. A repository that switched off a family on purpose with `ignore` was reported too.
pyrefly reads `pyrefly.toml` before `pyproject.toml`, and its preset `all` checks what `strict`
checks (<https://pyrefly.org/en/docs/configuration/#preset>). pytest 9 reads `pytest.toml`,
`.pytest.toml`, `pytest.ini`, `.pytest.ini`, `pyproject.toml`, `tox.ini` and `setup.cfg`, the first
that matches (<https://docs.pytest.org/en/stable/reference/customize.html>). The package managers keep their
cooldown in YAML (`pnpm-workspace.yaml`, `.yarnrc.yml`), in INI (`.npmrc`) or in a folder
(`.cargo/config.toml`), and each writes a time span its own way. An active external rule was also
checked in a project that does not use its tool: a rule of a Python linter in a repository of Go,
the cooldown of pnpm in a repository that installs with npm.

## Decision

1. **botica honors the configuration of the tool.** A setting that the tool applies holds the
   fragment wherever and however the repository writes it. A key that the repository sets to a
   value that holds less than the fragment asks (a shorter cooldown, `preset = "basic"`,
   `exclude-newer = false`, a family of ruff in `ignore`) is its decision, written on purpose:
   `external-config` does not report it. What `external-config` reports is a key the settings do
   not set, or an item that a list of the settings lacks. To leave out the whole rule, the
   repository switches the external rule off in the configuration of botica.
2. **botica gives the weaker values to another rule.** `external_config::weakened` returns, for each
   value of the settings that holds less than a fragment, the rule, the tool, the file the tool
   reads, the key from the table of the tool (`exclude-newer`, `errors.explicit-any`, `lint.select`
   for a code of ruff) and both values written as TOML. `external-config` does not use it; a rule
   of its own reports it as a warning that a repository switches off.
3. **An external rule asks nothing of a project that does not use its tool.** An external
   implementation may list in `applies-when` globs from the root of the project (`pnpm-lock.yaml`,
   `uv.lock`, `**/Dockerfile`, `**/*.tf`; `*` stays inside a folder and `**/` matches any folders,
   the root too). The project uses the tool when one of its files, as `botica lint` walks them,
   matches one. Without `applies-when`, a file of one of the `languages` of the implementation
   decides, by its extension; `*`, or a language whose files botica does not know, counts as used.
   `botica recipe check` leaves out the fragments of the tools that the project does not use too.
4. **A table of the tools that botica knows,** in `crates/botica-core/src/external_config/tools.rs`.
   For each tool it lists the files the tool reads its settings from, in the order the tool prefers
   them, with the table of each file that holds them; the key that names a file the settings extend
   (`extend` of ruff, `extends` of pyrefly), which botica follows inside the project; and the keys
   whose value is a time span. The first file that counts is the one botica reads, as the tool
   does: a `pyproject.toml` without the table of the tool does not count, and `ruff.toml` or
   `pytest.ini` count whenever they exist. A fragment for a file that is not in the table, or of a
   tool that is not, is read from that file alone.
5. **ruff and pyrefly have a reading of their own.** For ruff, a selector under `select` or
   `extend-select` of the fragment names the rules that ruff 0.17.1 has, from
   `crates/botica-core/src/external_config/ruff_rules.tsv`: a code names itself, and a family its
   stable rules, and also those in preview when the fragment turns on `preview` without
   `explicit-preview-rules`. Each of those rules holds when the most specific selector of the
   settings that takes it in is under `select` or `extend-select`, from the layer of highest
   priority that has `select`; a family written code by code holds the family. With
   `explicit-preview-rules = true` in the settings, a rule in preview holds only when a selector is
   its code, as ruff turns it on (`select = ["DOC"]` does not turn on `DOC102`). Without any
   `select` ruff applies its default rules, which changed in 0.16 and change again between minor
   versions (<https://docs.astral.sh/ruff/versioning/>), and botica does not count them. For
   pyrefly, a preset holds one that checks less (`all` holds `strict`, `strict` holds `default`),
   and a kind of error under `errors` holds when the repository sets it at least as high or the
   preset is `all`.
6. **A time span holds a span at least as long.** botica reads the forms that the tools take: a
   bare number in the unit of the key (minutes for pnpm and Yarn, days for npm, seconds for Bun),
   ISO 8601 (`P7D`), and numbers followed by units (`7 days`, `1 week`, `1w`). A date or `false`
   is not a span, and is a value the repository set on purpose (decision 1).
7. **A fragment is in the format of its file.** `config` of an external implementation is
   `config/<path>`: a TOML, YAML or INI file, under folders when the file of the project is in one.
   botica reads each of them into a TOML table, so the fragment and the file compare the same way.
   In an INI file every value is text, and `"true"` holds `true`. YAML is read with
   `serde-saphyr`.

## Considered options

- **botica asks the tool** (`ruff check --show-settings`, `npm config get`). Rejected. The tool
  becomes a dependency of every `botica lint`: a repository runs ruff through `uvx` in a hook of prek
  and may not have it on the `PATH`, and a run without network fails. `ruff check --show-settings`
  starts ruff on every run of `botica lint`, and its output is not a stable interface: ruff "does not yet have a stable API". pyrefly has no command that prints the severity
  of each kind of error (`pyrefly dump-config` prints the files and the paths of import). Asking
  pytest for its settings imports `conftest.py`, which runs code of the repository. ADR 0003 left
  running the tools to prek.
- **A declarative model in `rule.toml`** (the alternative files, the equivalent keys and the keys that
  switch off, for each rule). Rejected. The knowledge belongs to the tool and not to the rule: each
  rule of ruff would repeat the same files and keys. What ruff and pyrefly mean by a selector or a
  preset needs code anyway, so the model would grow a language of matchers.
- **Report a weaker value as an error.** Rejected: the repository wrote it on purpose, and its only
  answer would be to switch off the whole rule. A warning of another rule says it, and the
  repository switches that one off or silences it.
- **Count the default rules of ruff.** Rejected: the list changes with minor versions of ruff, and
  botica does not know which version the hook runs.
- **`applies-when` as languages only.** Rejected: a lockfile tells which package manager a project
  uses, and a Dockerfile or a file of Terraform has no extension of a language that botica knows.

## Consequences

- `botica-catalog` reads YAML with `serde-saphyr`, and parses INI with code of its own.
  `botica-catalog` and `botica-core` use `globset` for `applies-when`.
- A rule of ruff whose fragment lists codes is satisfied by `ignore` of those codes, and a rule
  whose fragment sets a value is satisfied by a weaker one. A standard that needs a value whatever
  the repository chooses is a rule of botica, not an external rule.
- `ruff_rules.tsv` is the list of ruff 0.17.1. A family written code by code against a newer ruff
  that added a rule to it holds until the list is made again; a code that the list does not have is
  read as a selector of its own.
- The table of tools changes when a tool changes where it reads its settings; a test pins each file
  and order with a link to the documentation of the tool.
- `botica lint` walks the files of the project once more when an active rule has an external
  implementation, to decide which tools the project uses.
- botica reads the root of the project. A setting in a nested `pyproject.toml` or in the
  configuration of the user (`~/.npmrc`, `$CARGO_HOME/config.toml`) is not seen, and options given
  on the command line of the hook (`ruff check --select`) are not seen either.
