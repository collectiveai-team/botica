The logger of a project is configured in one module, `core/logger.py`, and every other module gets
its logger from `get_logger`.

## What to read

- `_configure` picks the renderers from the `ENV` variable. With `prod` or `production` it writes one
  JSON object per event, and `dict_tracebacks` turns an exception into data. Otherwise it writes a
  console line, and the console renderer formats the exception itself. It reads the level once from
  `LOG_LEVEL` and adds the name of the logger to each event. `functools.cache` runs it once, on the
  first call to `get_logger`, so importing the module configures nothing.
- `get_logger` returns a `FilteringBoundLogger`. A module calls `get_logger(__name__)`, binds context
  with `.bind(request_id=rid)` and logs events as keyword pairs: `log.info("user_created",
  user_id=user.id)`.

## What to adapt

- The path of the module is `<your_package>/core/logger.py`.
- Keep the JSON and console split and the `LOG_LEVEL` variable. The standards of logging assume
  them.
- `logging.getLevelNamesMapping` needs Python 3.11 or newer.
