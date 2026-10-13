"""CES-74 · core-logger — the canonical house logger (structlog).

Drop this in at ``<your_package>/core/logger.py`` and import ``get_logger`` everywhere instead
of calling ``logging.getLogger`` (CES-45) or ``print`` in library code (CES-46).

Behaviour:
- **prod** (``ENV`` is ``prod``/``production``): structured JSON to stdout, machine-parseable.
- **dev** (anything else): colored, human-friendly console rendering.
- **level**: read once from ``LOG_LEVEL`` (default ``INFO``), case-insensitive.

Configuration is applied exactly once, lazily, on the first ``get_logger`` call. Bind context
with ``log = get_logger(__name__).bind(request_id=rid)`` and emit events as keyword pairs:
``log.info("user_created", user_id=user.id)``.

See ``.agents/rules/core-logger.md`` for the full rule.
"""

from __future__ import annotations

import logging
import os
import sys
from typing import TYPE_CHECKING

import structlog

if TYPE_CHECKING:
    from structlog.stdlib import BoundLogger

_configured = False


def _is_prod() -> bool:
    # Bootstrap infra: logging configures before the settings module exists, so it reads env
    # directly. Visible suppression keeps the CES-76 exception auditable.
    return os.getenv("ENV", "dev").lower() in {"prod", "production"}  # botica: ignore[settings-module]


def _level() -> int:
    name = os.getenv("LOG_LEVEL", "INFO").upper()  # botica: ignore[settings-module]
    return logging.getLevelNamesMapping().get(name, logging.INFO)


def _configure() -> None:
    global _configured
    if _configured:
        return

    shared = [
        structlog.contextvars.merge_contextvars,
        structlog.stdlib.add_log_level,
        structlog.processors.TimeStamper(fmt="iso", utc=True),
        structlog.processors.StackInfoRenderer(),
        structlog.processors.format_exc_info,
    ]
    renderer = (
        structlog.processors.JSONRenderer()
        if _is_prod()
        else structlog.dev.ConsoleRenderer(colors=True)
    )

    logging.basicConfig(format="%(message)s", stream=sys.stdout, level=_level())
    structlog.configure(
        processors=[*shared, renderer],
        wrapper_class=structlog.make_filtering_bound_logger(_level()),
        logger_factory=structlog.stdlib.LoggerFactory(),
        cache_logger_on_first_use=True,
    )
    _configured = True


def get_logger(name: str | None = None) -> BoundLogger:
    """Return the house-configured structlog logger (configuring the stack on first use)."""
    _configure()
    return structlog.get_logger(name)
