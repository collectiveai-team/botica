---
title: The Makefile holds docker orchestration targets only — not lint, test, or format
section: infra
scope: general
applies-to: all
status: current
tags: makefile, docker, infra, orchestration
---

## The Makefile holds docker orchestration targets only — not lint, test, or format

`Makefile` targets are docker orchestration shortcuts: build, start, stop, run-demo. Lint,
test, and format are run directly via `uv run ruff`, `uv run pytest`, `uvx prek run`, etc.
Env vars are loaded from `.env` with the `$(shell touch .env); include .env; export` pattern
so a missing `.env` never breaks the file.

**Prefer:**

```makefile
$(shell touch .env)
include .env
export $(shell sed 's/=.*//' .env)

core-build:
	docker compose build my-service-core

db-start:
	docker compose up -d postgres

run-demo-local:
	$(MAKE) start-ollama
	$(MAKE) start-rabbitmq
	$(MAKE) build-asr
	$(MAKE) start-asr
```

**Avoid:**

```makefile
lint:
	ruff check .    # don't add lint/test/format targets — run tools directly

test:
	pytest          # same — not a Makefile concern
```

Reference: see `infra-docker-first` for the image structure these targets build.
