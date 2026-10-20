---
id: CES-17
title: API boundary layout
---

## Directive

All inbound HTTP lives in one `api` package with a fixed, versioned shape. Request and response
schemas live under `api/<version>/schemas/{requests,responses}/`, and routers and endpoints under
`api/<version>/routers/`. A router module defines no schema. A request schema is named
`<Thing>Request` and a response schema `<Thing>Response`. The `api` package is the only inbound
boundary: `core` and `database` never import it.

## Why

The rules of botica read the layer of a file from its path: CES-4 (`api-schemas-extra-forbid`)
reports only models under `api/**/schemas/`, and leaves internal models alone. A versioned
subpackage (`v1`, `v2`) makes a breaking change of the API a new directory, and the modules that the
versions share stay as they are.
