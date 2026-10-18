The environment and the flags are read in one module of the package, `settings.py`, through a
`pydantic_settings.BaseSettings` subclass. The rest of the code gets the values from
`get_settings()`.

## What to read

- `Settings` reads the variables with the prefix `APP_` and the `.env` file. `case_sensitive=False`
  binds `DEBUG` and `debug` to the same field, and `extra="ignore"` skips the variables that the
  class does not declare.
- `get_settings` is cached with `lru_cache`: the settings are built and validated once per process.
- The comment at the end shows `os.getenv` outside the module, which the rule reports.

## What to adapt

- The fields and the prefix are an example. Declare the variables of your package, with their types
  and defaults.
- Each layer or package has its own `settings.py`.
