---
title: Write Prefect Markdown artifacts to make LLM I/O inspectable in the UI
section: prefect
scope: specific
applies-to: prefect
status: current
tags: prefect, artifacts, llm, observability, debugging
---

## Write Prefect Markdown artifacts to make LLM I/O inspectable in the UI

Any task that calls an LLM or extraction pipeline should write a Prefect Markdown
artifact capturing the model name, settings, system prompt, user prompt, and
response. This surfaces the full context of every AI call directly in the Prefect
UI, making debugging and auditing possible without touching logs or storage. Use a
shared `generate_agent_artifact` helper so the format stays consistent across tasks.

**Prefer:**

```python
from prefect.artifacts import create_markdown_artifact

# in a @task, after the LLM call
if artifact_key:
    await generate_agent_artifact(
        key=artifact_key,           # e.g. "parties_extraction-contract.pdf"
        model=agent.model.model_name,
        model_settings=agent.model_settings,
        system_prompt=system_prompt,
        user_prompt=text,
        response=result.model_dump_yaml(),
    )
```

**Avoid:**

```python
# silently returning structured output with no trace of what the LLM saw or said
result = await agent.run(text)
return result.output
```

A `generate_agent_artifact` helper wrapping `create_markdown_artifact` keeps the format
consistent: model settings (YAML), system prompt, and paired user prompt / response sections.
Every extraction task that accepts an `artifact_key` should call this helper after the agent
run.

Reference: Prefect docs — `prefect.artifacts.create_markdown_artifact`.
