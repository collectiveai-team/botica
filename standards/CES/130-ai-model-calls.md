---
id: CES-130
title: Calls to an AI model return validated output
references: ["ai"]
---

## Directive

Code that calls a language model or an embedding model gets back a value of a type it declared,
checked by the library that makes the call.

- A call whose answer the code reads as data declares the type of the answer, such as a pydantic
  model in Python or a zod schema in TypeScript, and the library validates the answer against it.
  The code does not take the data out of the text of the answer with `json.loads`, a regular
  expression or a split. An answer that is text for a person, such as a summary or a reply in a
  chat, is a `str`.
- The SDK of a provider is called only for what the library does not offer, such as an API of the
  provider that the library has no model class for. The calls to it are in one module, and a
  comment there names the feature.
- A prompt is a file of the package, read at run time, and the name of the model is a setting.

In Python the library of choice is pydantic-ai: an `Agent` for a run with an output type, tools or
a stream, `pydantic_ai.direct.model_request` for one request that needs none of them, and an
`Embedder` for embeddings. Another library that validates the answer against the declared type
meets this standard too.

## Why

A model returns text, and text that looks like JSON can miss a field, add one or carry a value of
the wrong type. A declared type turns that into an error at the call, where the library can send it
back to the model. The page "Retries" of pydantic-ai says that the retry message carries "a list of
Pydantic error details (from a ValidationError), and renders for the model with 'Fix the errors and
try again.' appended". Code that parses the text itself finds the same problem later, in another
function, as a `KeyError` or a wrong value.

pydantic-ai names a model as `"<provider>:<model>"`, so changing the provider is a change of a
setting and not of the code, and its `TestModel`, `FunctionModel` and `TestEmbeddingModel` stand
for the provider in tests with no key and no network. The SDK of a provider ties the code to that
provider and its tests to mocks of its client. Keeping those calls in one module keeps that tie in
one place.

A prompt in a file is reviewed in a diff like code, and the name of the model in the settings is
changed per environment without a release.

## Examples

```python
class Parties(BaseModel):
    parties: list[Party]


agent = Agent(settings.llm_model, instructions=load_prompt("extract_parties"), output_type=Parties)
parties = (await agent.run(contract)).output
```

```python
response = client.chat.completions.create(model="gpt-5.5", messages=messages)
parties = json.loads(response.choices[0].message.content)["parties"]
```

The first call gets a `Parties` that pydantic validated, with the prompt from a file and the model
from the settings. The second parses the text of the answer by hand, through the SDK of one
provider, with the model written in the code, and a missing key fails one line later.
