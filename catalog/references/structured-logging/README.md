The logger of a project is configured in one module, `core/logger.py`, and every other module gets
its logger from `get_logger`.

## What to read

- `_configure` picks the renderer from the `ENV` variable: JSON when it is `prod` or `production`,
  a colored console otherwise. It reads the level once from `LOG_LEVEL`. It runs on the first call
  to `get_logger`, so importing the module configures nothing.
- `get_logger` returns the structlog logger. A module calls `get_logger(__name__)`, binds context
  with `.bind(request_id=rid)` and logs events as keyword pairs: `log.info("user_created",
  user_id=user.id)`.

## What to adapt

- The path of the module is `<your_package>/core/logger.py`.
- Keep the JSON and console split and the `LOG_LEVEL` variable. The standards of logging assume
  them.
