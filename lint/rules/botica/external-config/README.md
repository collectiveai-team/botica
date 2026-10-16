## Directive

An external rule is enforced by another tool, and the rule names the fragment of configuration the
tool needs and the file of the project it goes into (`config = "config/prek.toml"` in its
`rule.toml` goes into `prek.toml`). For each active external rule, `botica lint` reads that file at
the root of the project and reports it, as a whole, when it does not exist, is not TOML, or lacks
the fragment. The check runs on every `botica lint`, whatever files it is given.

A file holds a fragment when it has each key of the fragment with a value that holds the value of
the fragment: a table holds the keys it names, an array holds one item for each item of the
fragment, and any other value is equal. `botica explain <rule>` shows the configuration of an
external rule.

## Why

A standard enforced by a tool holds only while the tool is set up: a repository that drops the hook
of complexipy from `prek.toml` stops measuring cognitive complexity, and no check fails.

## Suppression

The finding is about a whole file of configuration. Switch the external rule off, or this one with
`ignore = ["external-config"]` in the configuration of botica.
