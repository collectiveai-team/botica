A call to a language model or an embedding model goes through pydantic-ai, and a call to a
language model takes its prompt from a file of the package. A call that returns a structure gets
a validated pydantic model from a `pydantic_ai.Agent`; a call that returns text, whole or as it
streams, gets a `str` from the same `Agent`; a call that turns text into vectors goes through a
`pydantic_ai.Embedder`. pydantic-ai is the library of choice for Python (CES-130); another library
that validates the answer against a declared type does the same job.

## What to read

- The prompts are `app/prompts/extract_parties.md` and `app/prompts/summarize.md`, Markdown files
  next to the code. A change to one is reviewed and versioned like a change to the code.
- `load_prompt` reads them with `importlib.resources.files`, which finds the file in an installed
  wheel as well as in a checkout, and caches it for the process.
- `parties_agent` builds the agent with the prompt as `instructions` and `Parties` as
  `output_type`. pydantic-ai validates the answer of the model against `Parties` and, when the
  validation fails, sends the error back to the model, up to `retries` times.
- `extract_parties` runs the agent and returns `result.output`, already a `Parties`.
- `summary_agent` builds an agent with `output_type=str`: the run ends when the model answers with
  text, and `result.output` is that text. `summarize` waits for the whole answer.
- `stream_summary` opens `agent.run_stream` and yields each piece of the text as the model writes
  it, with `stream_text(delta=True)`. The default `debounce_by=0.1` groups the pieces that arrive in
  0.1 seconds; `None` yields each one as it arrives.
- `passage_embedder` builds an `Embedder` from the name of an embedding model.
  `embed_passages` calls `embed_documents` for the text that is stored and searched, and
  `embed_question` calls `embed_query` for the text that searches it: some models embed the two
  differently.
- The tests drive the agents with a `FunctionModel` (`stream_function` for the stream) and the
  embedder with a `TestEmbeddingModel`, which stand for the provider: no key and no network.
  `Embedder.override(model=...)` and `Agent.override(model=...)` replace the model that the
  settings name.

## What to adapt

- The prompts, `Party`, `Parties`, the summary and the passages are examples: one prompt file and
  one output type per task.
- The model and the embedding model are settings (`APP_LLM_MODEL`, `APP_EMBEDDING_MODEL`, read by
  the settings module), passed as a name such as `"openai:gpt-5.5"` or
  `"openai:text-embedding-3-small"`, or as a `Model` or an `EmbeddingModel`. The provider is the
  prefix of the name, and its extra of `pydantic-ai-slim` (`pydantic-ai-slim[openai]`) is a
  dependency of the project.
- A call that needs neither tools nor validation of the output, such as one request with its own
  messages, can use `pydantic_ai.direct.model_request`, which takes the same model names.
- Call the SDK of a provider only for what pydantic-ai does not offer, such as an API of the
  provider that has no model class in pydantic-ai. Keep that call in one module, with a comment
  that names the feature.
- Do not name a prompt file `*.prompt.yml`: that is the format of GitHub Models, and `gh models`
  reads such files with its own schema.
- Check that the build of the package includes the prompt files.
