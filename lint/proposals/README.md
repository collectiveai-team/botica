# Proposed rules

Rules that botica could measure and does not yet. Each folder has the shape of a folder of
`lint/rules`: `rule.toml`, the YAML, a `README.md` and `fixtures/` with an `expected.json`.
**Nothing here is in the catalog**: botica does not read this directory (`build.rs` embeds only
`lint/rules`), the conformance runner does not run it, and no hook selects it.

There are no proposed rules today. The six that were here joined the catalog in `0.10.0` as
`preview` rules: `BTFAST002`–`BTFAST004` (`api-schema-in-route-module`, `api-request-schema-name`,
`api-response-schema-name`) and `BTARCH003`–`BTARCH005` (`settings-base-class`,
`settings-direct-instantiation`, `db-no-create-all`).

## Integrating one

1. `git mv lint/proposals/<family>/<slug> lint/rules/<family>/<slug>`, and set `since` to the next
   version.
2. Add the slug to the lists that name the catalog: `botica-cli/tests/conformance.rs`,
   `botica-config/tests/resolve_shipped_catalog.rs` and the table of `lint/README.md`. Decide with
   the team whether the hook selection of `scaffolding/templates/prek-generic.toml` takes it.
3. Run the tests: the conformance runner checks the fixtures with the four variants.

`tests/test_catalog_is_the_only_doc.py` fails if a slug or a code is in both places, so a rule is
moved, never copied.
