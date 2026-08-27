---
title: Dev and runtime are Docker-first — devcontainer extends the core image
section: infra
scope: general
applies-to: all
status: current
tags: docker, devcontainer, infra, model-preload
---

## Dev and runtime are Docker-first — devcontainer extends the core image

Every repo ships a `.devcontainer/` alongside a `docker-images-build/` or `docker-images/`
directory. The devcontainer `Dockerfile` extends the same core image used at runtime, so dev
and prod share a base. Heavy models are baked in at image build time — not pulled on first
inference — so containers start ready.

**Prefer (devcontainer FROM the runtime core; model preloaded at build):**

```dockerfile
# .devcontainer/Dockerfile
FROM my-service-core:latest AS my-service-dev

# docker-images/my-service/Dockerfile — model pulled during build
RUN uv run python -c "import whisper; whisper.load_model('${WHISPER_MODEL}')"
```

**Avoid:**

```dockerfile
# Don't pull models at container start — cold-start latency blooms into a runtime error
CMD python -c "import whisper; whisper.load_model('base.en')" && uvicorn main:app
```

Reference: see `infra-makefile-docker` for how Makefile targets drive these builds.
