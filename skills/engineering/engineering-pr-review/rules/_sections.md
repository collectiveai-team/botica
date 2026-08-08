# Sections

This file defines every rule section: its id (the filename prefix), ordering, scope, and
the atomic rules that belong to it. The section id in parentheses is the filename prefix
(`general-respect-local-repo.md` belongs to section `general`).

Rules carry `applies-to` frontmatter (`all` or a stack tag like `nextjs`, `prefect`,
`fastapi`, `asr`, `llm`, `k8s`) so the agent can tell a universal rule from a stack-specific
one. The pack defines one standard per concern and marks where existing code should head.

---

## 1. General Baseline (general)

**Scope:** general — applies everywhere.
**Description:** Cross-repo defaults: respect local contracts, repo shape, internal deps.

- `general-respect-local-repo` — don't bulldoze a working repo's contracts in an unrelated change
- `general-repo-shape` — package layout; keep notebooks/resources/reports out of the package
- `general-internal-git-deps` — pin internal/shared deps by git tag, never a branch

## 2. Python Toolchain (py)

**Scope:** general.
**Description:** One standard toolchain — uv + ruff + ruff-format + pyrefly + ast-grep, run
through prek, line 100, latest Python.

- `py-package-manager` — use uv; commit the lockfile; migrate Poetry/pip repos to it
- `py-build-backend` — use hatchling (+ hatch-vcs for git-tag versioning)
- `py-version-support` — target the latest Python for new code; honor an existing repo's pin
- `py-ruff-format-modern` — ruff + ruff-format + pyrefly + ast-grep via prek, line 100
- `py-legacy-lint-stack` — recognize a legacy black/isort/pylint/mypy setup and migrate it
- `py-no-formatter-churn` — never reformat outside the touched area

## 3. Python Layout & Design (pylayout)

**Scope:** general — the shared package-shape conventions.
**Description:** The package slice conventions.

- `pylayout-meta-slice` — `meta/` holds ABC interfaces + Pydantic domain models
- `pylayout-adapters-slices` — concrete implementations live in sibling slices, not in meta/
- `pylayout-pydantic-v2` — Pydantic v2 for domain data; pydantic-settings for app config
- `pylayout-cli-typer-rich` — Typer + Rich for CLI entrypoints
- `pylayout-src-layout` — use src/ only if the repo already does; avoid double-nesting

## 4. Architecture: Deep Modules (arch)

**Scope:** general. **Source:** Matt Pocock `codebase-design` (vendored, attributed).
**Description:** Shared architecture vocabulary and the deletion/adapter discipline.

- `arch-deep-modules` — module/interface/depth/seam/adapter/leverage/locality; depth-as-leverage
- `arch-deletion-test` — delete the module: does complexity vanish (pass-through) or spread (earning)?
- `arch-adapter-discipline` — one adapter = hypothetical seam, two = real
- `arch-interface-is-test-surface` — test observable behavior through the same seam callers cross
- `arch-vocabulary` — use the exact terms; avoid component/service/API/boundary

## 5. Large Files & Spaghetti (spaghetti)

**Scope:** general. **Source:** a pragmatic house heuristic — NOT Matt Pocock (he rejects line ratios).
**Description:** Concrete size/mixing heuristics for PR gating.

- `spaghetti-large-file-thresholds` — 400 lines = large, 700 = enormous (house heuristic)
- `spaghetti-mixed-orchestration` — don't pile transport+validation+domain+persistence+formatting in one file
- `spaghetti-no-utils-dumping` — no catch-all `utils.py` / `helpers.py`
- `spaghetti-function-size` — split functions over ~80 lines or with deep nesting

## 6. Testing (test)

**Scope:** general.
**Description:** pytest baseline, in-memory adapters, treat untested code as a gap.

- `test-pytest-config` — pytest + the shared `[tool.pytest.ini_options]` (testpaths, filterwarnings)
- `test-in-memory-adapters` — in-memory SQLite + TestClient; no live services
- `test-through-interface` — assert behavior at the module interface, not internals
- `test-coverage-gap` — treat untested code as a gap; add tests on behavior change

## 7. Logging (log)

**Scope:** general.
**Description:** The get_logger + Rich convention; prod observability.

- `log-get-logger-rich` — `get_logger(__name__)` = stdlib logging + RichHandler, level from `LOG_LEVEL`
- `log-no-print` — libraries log, they don't `print`
- `log-observability` — Logfire instrumentation in prod services

## 8. FastAPI (api)

**Scope:** specific — `applies-to: fastapi`.
**Description:** API structure, settings, ORM, schemas, auth.

- `api-structure` — app construction / routers / services split; lifespan for startup/shutdown
- `api-pydantic-settings` — `get_settings()` lru_cache singleton; never instantiate Settings() directly
- `api-sqlmodel-alembic` — SQLModel + Alembic; from_attributes; migrations at startup
- `api-schemas` — explicit request/response Pydantic models, ConfigDict(extra="forbid")
- `api-auth` — short-lived JWT (default) or OAuth/OIDC for SSO; avoid long-lived/localStorage tokens

## 9. LLM & AI (llm)

**Scope:** specific — `applies-to: llm`.
**Description:** The LLM application stack.

- `llm-pydantic-ai-structured` — pydantic-ai Agent with `output_type` for structured output
- `llm-no-langchain` — prefer openai / pydantic-ai; avoid LangChain
- `llm-diskcache` — cache LLM responses with diskcache, keyed by hash of prompt+input+model
- `llm-prompts-yaml` — prompts as versioned `.prompt.yml` files, co-located with task code
- `llm-observability` — Logfire (prod) / Langfuse (experiments)
- `llm-vector-stores` — Chroma/Weaviate behind an ABC; OpenAI/Ollama embeddings; embedding cache

## 10. Prefect (prefect)

**Scope:** specific — `applies-to: prefect`.
**Description:** Workflow orchestration.

- `prefect-flows-tasks` — `@flow`/`@task`, async, `log_prints`, `get_run_logger`
- `prefect-serve-deploy` — `prefect.serve(*deployments)` in deploy.py; Python deployments, no YAML
- `prefect-artifacts` — `create_markdown_artifact` for UI-inspectable task I/O
- `prefect-webhook-hooks` — lifecycle hooks → webhook (workaround for 3.x automations)
- `prefect-api-bridge` — `run_deployment` + polling from the API; wait_and_cleanup
- `prefect-parallel-tasks` — `asyncio.gather` for independent tasks within a flow

## 11. ASR & Streaming (asr)

**Scope:** specific — `applies-to: asr`.
**Description:** Real-time ASR over websockets.

- `asr-websocket-server` — FastAPI single `/ws` endpoint; per-connection manager via Depends
- `asr-streaming-interfaces` — ASRStreamingInterface / VADModelInterface / ChunkPolicyInterface ABCs in meta/
- `asr-vad` — Silero (torch.hub) / WebRTC VAD; thresholds via env
- `asr-threading-bridge` — threading + queue.Queue bridging the async handler (real pattern + caveats)
- `asr-models` — Whisper / NeMo / Deepgram / whisper-trt; model selection via env; edge targets

## 12. Frontend / Next.js (fe)

**Scope:** specific — `applies-to: nextjs`.
**Description:** The frontend stack and its real (SPA-leaning) shape vs the direction.

- `fe-app-router` — App Router; route-segment structure; co-locate page components
- `fe-shadcn-tailwind` — shadcn/ui + Tailwind + `cn()`; components.json; lucide icons
- `fe-server-components-direction` — client-heavy SPA is common; direction = server-components-first
- `fe-data-fetching` — fetch on the server (Server Component / proxy Route Handler); keep secrets server-side
- `fe-auth` — provider-backed session (OAuth) with httpOnly cookies; avoid localStorage JWT
- `fe-tooling` — strict tsconfig; one package manager per repo; don't ship with build checks disabled

## 13. Containers & Deploy (infra)

**Scope:** general.
**Description:** Docker-first dev, Makefile-as-docker, CI release-on-tag.

- `infra-docker-first` — `.devcontainer/` + `docker-images*/`; build-time model preload
- `infra-makefile-docker` — the Makefile holds docker orchestration targets, not lint/test
- `infra-compose-traefik` — docker compose (+ Traefik for prod)
- `infra-ci-release-on-tag` — CI builds/releases on `v*` tags; run pytest in CI
- `infra-dual-target-builds` — x86 + Jetson dual Dockerfiles for edge targets

## 14. Kubernetes (k8s)

**Scope:** specific — `applies-to: k8s`.
**Description:** Don't ship bare Deployments; harden toward production-readiness.

- `k8s-current-gap` — manifests lacking probes/limits/HPA are a gap to harden
- `k8s-resource-limits-probes` — set requests+limits and readiness/liveness/startup probes
- `k8s-security-context` — runAsNonRoot, drop capabilities, read-only rootfs

## 15. Pull Requests (pr)

**Scope:** general — the PR gate.
**Description:** What a PR must satisfy before it opens.

- `pr-small-by-intent` — small enough to review by intent, not just file count
- `pr-description` — state why it exists, what changed, how it was verified
- `pr-complexity-check` — inspect added complexity (shallow modules, large-file growth, new deps) before opening
- `pr-run-checks` — run the repo's checks; report any that couldn't run
