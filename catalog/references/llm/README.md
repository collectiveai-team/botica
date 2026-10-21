A call to a language model takes its prompt from a file of the package and returns a validated
pydantic model, through a `pydantic_ai.Agent`.

## What to read

- The prompt is `app/prompts/extract_parties.md`, a Markdown file next to the code. A change to it
  is reviewed and versioned like a change to the code.
- `load_prompt` reads it with `importlib.resources.files`, which finds the file in an installed
  wheel as well as in a checkout, and caches it for the process.
- `parties_agent` builds the agent with the prompt as `instructions` and `Parties` as
  `output_type`. pydantic-ai validates the answer of the model against `Parties` and, when the
  validation fails, sends the error back to the model, up to `retries` times.
- `extract_parties` runs the agent and returns `result.output`, already a `Parties`.
- The tests drive the agent with a `FunctionModel`, which stands for the provider: no key and no
  network.

## What to adapt

- The prompt, `Party` and `Parties` are one example: one prompt file and one output model per task.
- The model is a setting (`APP_LLM_MODEL`, read by the settings module), passed to the agent as a
  name such as `"openai:gpt-5.5"` or as a `Model`.
- Do not name a prompt file `*.prompt.yml`: that is the format of GitHub Models, and `gh models`
  reads such files with its own schema.
- Check that the build of the package includes the prompt files.
