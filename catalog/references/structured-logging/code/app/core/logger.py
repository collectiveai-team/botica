"""The logger of the package: structlog, configured once, on the first call to `get_logger`."""

from __future__ import annotations

import functools
import logging
import os
import sys
from typing import TYPE_CHECKING

import structlog

if TYPE_CHECKING:
    from structlog.typing import FilteringBoundLogger, Processor


def _is_prod() -> bool:
    # Logging is configured before the settings module is read, so it reads the environment itself.
    env = os.getenv("ENV", "dev")  # botica: ignore[settings-module]
    return env.lower() in {"prod", "production"}


def _level() -> int:
    name = os.getenv("LOG_LEVEL", "INFO").upper()  # botica: ignore[settings-module]
    return logging.getLevelNamesMapping().get(name, logging.INFO)


@functools.cache
def _configure() -> None:
    shared: list[Processor] = [
        structlog.contextvars.merge_contextvars,
        structlog.stdlib.add_logger_name,
        structlog.stdlib.add_log_level,
        structlog.processors.TimeStamper(fmt="iso", utc=True),
        structlog.processors.StackInfoRenderer(),
    ]
    renderers: list[Processor] = (
        [structlog.processors.dict_tracebacks, structlog.processors.JSONRenderer()]
        if _is_prod()
        else [structlog.dev.ConsoleRenderer()]
    )
    level = _level()
    logging.basicConfig(format="%(message)s", stream=sys.stdout, level=level)
    structlog.configure(
        processors=[*shared, *renderers],
        wrapper_class=structlog.make_filtering_bound_logger(level),
        logger_factory=structlog.stdlib.LoggerFactory(),
        cache_logger_on_first_use=True,
    )


def get_logger(name: str | None = None) -> FilteringBoundLogger:
    """Return the logger of `name`, usually `__name__`, configuring structlog on the first call."""
    _configure()
    return structlog.get_logger(name)
