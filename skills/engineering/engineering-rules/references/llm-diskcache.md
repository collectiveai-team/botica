---
title: Cache LLM responses with diskcache keyed by SHA-256 of prompt + input + model
section: llm
scope: specific
applies-to: llm
status: current
tags: llm, cache, diskcache, sha256, performance
---

## Cache LLM responses with diskcache keyed by SHA-256 of prompt + input + model

LLM API calls are expensive and deterministic at `temperature=0.0`. Cache every
response using `diskcache.Cache`, keyed by `SHA-256(system_prompt + user_input + model)`.
This avoids redundant calls during re-runs, retries, and development iteration.

**Prefer:**

```python
import hashlib
import diskcache

cache = diskcache.Cache("/var/cache/llm/diskcache")

def cache_key(system_prompt: str, text: str, model: str) -> str:
    digest = hashlib.sha256((system_prompt + text + model).encode()).hexdigest()
    return f"llm_{digest}"

# Pass the cache and key to your agent wrapper so it checks before calling the model
agent = Agent(
    model=model,
    output_type=ClausesExtractionResult,
)
key = cache_key(system_prompt, text, model)
if key in cache:
    result = cache[key]
else:
    result = await agent.run(text)
    cache[key] = result
```

**Avoid:**

```python
# Calling the model on every run without caching — wasteful for identical inputs
result = await agent.run(text)
```

The embedding cache (for vector store lookups) is a separate concern — see
`llm-vector-stores`.
