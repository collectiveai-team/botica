---
id: CES-131
title: A pull request that breaks a published interface carries the label breaking-change
---

## Directive

A published interface is one that code outside the repository depends on: the public API of a
package that others import, the HTTP contract of a service (its OpenAPI document) and the events it
emits (its AsyncAPI document). A pull request is compared with its base, and a change that breaks a
consumer of the interface fails the check unless the pull request carries the label
`breaking-change`. Either way the check leaves the break on the pull request as an annotation: a
warning when the label is there, an error when it is not. The release that carries the change takes
a major version, or a minor one below 1.0.

| Interface | Tool | Recipe |
| --- | --- | --- |
| Python package | [`griffe check`](https://mkdocstrings.github.io/griffe/guide/users/checking/) | `ci-api-compat-python` |
| OpenAPI document | [`oasdiff breaking`](https://github.com/oasdiff/oasdiff) | `ci-openapi-compat` |
| AsyncAPI document | [`asyncapi diff --type breaking`](https://www.asyncapi.com/docs/tools/cli/usage#asyncapi-diff-old-new) | `ci-asyncapi-compat` |
| Rust crate | [`cargo-semver-checks`](https://github.com/obi1kenobi/cargo-semver-checks) | none yet |
| Go module | [`gorelease`](https://pkg.go.dev/golang.org/x/exp/cmd/gorelease), [`apidiff`](https://pkg.go.dev/golang.org/x/exp/cmd/apidiff) | none yet |
| TypeScript package | [API Extractor](https://api-extractor.com/) | none yet |

A repository that publishes no interface, such as a service that only its own frontend calls,
does not need the check.

The OpenAPI and AsyncAPI documents are generated from the code and not committed: the check exports
them at the head and at the base of the pull request, in the same environment. When the base has no
package or no app to export, as in the pull request that creates it, there is nothing to compare and
the check passes.

## Why

Ruff, Pyrefly and the tests check the code against itself. A renamed parameter or a removed
response field passes all three, and breaks every caller outside the repository on the next
release. The comparison with the base finds the change in the pull request that makes it, and the
label tells the reviewer and the release notes that the break is on purpose.

## Limits

- griffe takes as public the names without a leading `_`; a package that declares its API only
  with `__all__` gets false findings. It does not compare return types.
- oasdiff compares the documents. A change of behaviour that keeps the document, or a field that
  the document does not declare (`response_model=None`), goes unseen.
- `asyncapi diff` classifies the changes inside a message payload as unclassified. The overrides
  that `ci-asyncapi-compat` installs make a removed or retyped property, and a property that stops
  being required, breaking, at the first level of the payload; a change in a nested object is
  still unclassified and passes.
- The OpenAPI document of a FastAPI app has no body for a streaming response, so an SSE or
  WebSocket stream is in the AsyncAPI document or nowhere.

## Suppression

A break that is on purpose carries the label `breaking-change`. The workflow is not switched off to
let one change through.
