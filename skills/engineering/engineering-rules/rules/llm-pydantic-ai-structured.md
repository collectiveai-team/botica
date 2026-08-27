---
title: Use pydantic-ai Agent with output_type for structured LLM output
section: llm
scope: specific
applies-to: llm
status: current
tags: llm, pydantic-ai, structured-output, agent
---

## Use pydantic-ai Agent with output_type for structured LLM output

Wrap every LLM call in a `pydantic_ai.Agent` with `output_type=<PydanticModel>`.
This makes structured extraction type-safe and validated at runtime. Use
`temperature=0.0` for deterministic extraction tasks; set `retries` to handle
transient model errors.

**Prefer:**

```python
from pydantic_ai import Agent
from pydantic_ai.settings import ModelSettings

agent: Agent[None, ClausesExtractionResult] = Agent(
    model=model,
    model_settings=ModelSettings(temperature=0.0),
    system_prompt=system_prompt,
    output_type=ClausesExtractionResult,   # validated Pydantic model
    retries=3,
)
result = await agent.run(text)
clauses: ClausesExtractionResult = result.output
```

**Avoid:**

```python
# Raw string completion — no schema enforcement, no retry logic
response = openai_client.chat.completions.create(
    model=model,
    messages=[{"role": "user", "content": prompt}],
)
data = json.loads(response.choices[0].message.content)  # fragile
```

Reference: `llm-diskcache` for caching LLM responses; `llm-prompts-yaml` for
loading `system_prompt` from co-located `.prompt.yml` files.
