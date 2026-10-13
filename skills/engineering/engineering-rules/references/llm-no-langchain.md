---
title: Do not introduce LangChain or LlamaIndex
section: llm
scope: specific
applies-to: llm
status: current
tags: llm, langchain, pydantic-ai, openai, ollama
---

## Do not introduce LangChain or LlamaIndex

Use **pydantic-ai** for structured agents, the **raw OpenAI SDK** for direct API
calls, and **Ollama** for local model inference. Do not introduce LangChain or
LlamaIndex — they fragment the abstraction layer and pull in a large dependency
tree for capabilities already covered by these primitives.

**Prefer (existing primitives):**

```python
# structured agents
from pydantic_ai import Agent

# direct SDK calls
from openai import OpenAI

# local inference
import ollama

# retrieval — via VectorStore ABC + ChromaStore / WeaviateStore
from your_package.interfaces.vector_store import VectorStore
```

**Avoid:**

```python
# Do not add to any project
from langchain.chat_models import ChatOpenAI
from langchain.chains import RetrievalQA
from llama_index.core import VectorStoreIndex
```

The primitives above cover structured output, retrieval, and agent orchestration
without the overhead of a framework layer.

Reference: `llm-pydantic-ai-structured` for the agent layer;
`llm-vector-stores` for retrieval abstractions.
