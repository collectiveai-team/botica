# Proposed rules

Rules that botica could measure and does not yet. Each folder has the shape of a folder of
`lint/rules`: `rule.toml`, the YAML, a `README.md` and `fixtures/` with an `expected.json`.
**Nothing here is in the catalog**: botica does not read this directory (`build.rs` embeds only
`lint/rules`), the conformance runner does not run it, and no hook selects it.

They are the deterministic halves of guidance that used to sit in the `engineering-rules` skill.
That guidance was removed from the skill, so a rule has one description only; until a rule is
integrated, its README here is where the text lives.

| Code | Rule | Replaces guidance of |
|---|---|---|
| `BTFAST002` | [`api-schema-in-route-module`](fastapi/api-schema-in-route-module/README.md) | `api-schemas`: schemas are not defined in route files |
| `BTFAST003` | [`api-request-schema-name`](fastapi/api-request-schema-name/README.md) | `api-schemas`: the `{Name}Request` convention |
| `BTFAST004` | [`api-response-schema-name`](fastapi/api-response-schema-name/README.md) | `api-schemas`: the `{Name}Response` convention |
| `BTARCH003` | [`settings-base-class`](architecture/settings-base-class/README.md) | `pylayout-pydantic-v2`: settings extend `BaseSettings` |
| `BTARCH004` | [`settings-direct-instantiation`](architecture/settings-direct-instantiation/README.md) | `api-pydantic-settings`: callers never build `Settings()` |
| `BTARCH005` | [`db-no-create-all`](architecture/db-no-create-all/README.md) | `api-sqlmodel-alembic`: never `metadata.create_all()` outside tests |

All six are `status = "preview"` and `default-enabled = false`, and none cites a CES. The codes are
the next free ones of their family and are only reserved when a rule is integrated. The
`expected.json` files come from the ast-grep CLI, like the ones of the catalog.

## Integrating one

1. `git mv lint/proposals/<family>/<slug> lint/rules/<family>/<slug>`.
2. Add the slug to the lists that name the catalog: `botica-cli/tests/conformance.rs`,
   `botica-config/tests/resolve_shipped_catalog.rs` and the table of `lint/README.md`. Decide with
   the team whether the hook selection of `scaffolding/templates/prek-generic.toml` takes it.
3. Run the tests: the conformance runner checks the fixtures with the four variants.

`tests/test_catalog_is_the_only_doc.py` fails if a slug or a code is in both places, so a rule is
moved, never copied.
