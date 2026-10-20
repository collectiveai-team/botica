The environment and the flags are read in one module of the package, `settings.py`, through a
`pydantic_settings.BaseSettings` subclass. The rest of the code gets the values from
`get_settings()`.

## What to read

- `Settings` reads the variables with the prefix `APP_` and the `.env` file. `case_sensitive=False`
  binds `APP_DEBUG` and `app_debug` to the same field. `extra="ignore"` ignores the entries of the
  `.env` file that the class does not declare; the variables of the process that it does not
  declare are ignored in any case.
- `get_settings` is cached with `lru_cache`: the settings are built and validated once per process.
  A test calls `get_settings.cache_clear()` after it changes the environment.
- An endpoint takes the settings with `Depends(get_settings)`, and a test replaces them with
  `app.dependency_overrides[get_settings]`. A module that calls `get_settings()` when it is imported
  keeps the first settings and cannot be overridden.

## What to adapt

- The fields and the prefix are an example. Declare the variables of your package, with their types
  and defaults.
- Each layer or package has its own `settings.py`.
