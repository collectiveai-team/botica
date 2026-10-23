## Directive

Cargo resolves only the versions that were published at least 7 days ago. `.cargo/config.toml`
sets `global-min-publish-age = "7 days"` in `[registry]`, and `cargo add`, `cargo update` and
`cargo generate-lockfile` then skip a newer version that `Cargo.lock` does not hold yet.

## Why

A malicious release has no advisory during its first days, and `osv-scanner` reports it only once
the advisory exists. The cooldown of Dependabot holds back the dependency that its pull request
updates, but not the transitive ones, and nothing holds back a package added by a person or an
agent.

## Exceptions

Cargo has no exception in its configuration. A version that `Cargo.lock` already holds stays, and
one version comes in with
`CARGO_RESOLVER_INCOMPATIBLE_PUBLISH_AGE=allow cargo update -p left-pad --precise 1.3.1`, with the
reason in the pull request.

## Tool

[Cargo](https://doc.rust-lang.org/nightly/cargo/reference/config.html#registryglobal-min-publish-age)
enforces the rule from Rust 1.100. Cargo 1.93 ignores the key without a warning, and a project on
an older toolchain relies on the cooldown of Dependabot. Botica checks that `.cargo/config` or
`.cargo/config.toml` has it, the one that Cargo reads.

## Configuration

```toml
[registry]
global-min-publish-age = "7 days"
```

Any span of 7 days or more holds the rule: `"1 weeks"` or `"14 days"`. A shorter span, `"0"` too,
is a value the project wrote on purpose, and botica does not report it. botica asks for the
setting only in a project with `Cargo.lock`.

## Do

```toml
[registry]
global-min-publish-age = "7 days"
```

## Don't

```toml
[registry]
global-min-publish-age = "0"
```

## Suppression

The finding is about the file as a whole. Switch the rule off with
`ignore = ["cargo-cooldown"]` in the configuration of botica.
