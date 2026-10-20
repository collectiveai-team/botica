"""The logger of the topic, run in a fresh interpreter for each environment it reads."""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path

CODE = Path(__file__).resolve().parent.parent / "code"

LOG_USER = """
from app.core.logger import get_logger

log = get_logger("app.users").bind(request_id="r-1")
log.debug("cache_miss")
log.info("user_created", user_id=7)
try:
    1 / 0
except ZeroDivisionError:
    log.exception("user_failed")
"""


def run(**environment: str) -> str:
    env = {"PATH": os.environ.get("PATH", ""), "PYTHONPATH": str(CODE), **environment}
    done = subprocess.run(
        [sys.executable, "-c", LOG_USER], env=env, capture_output=True, text=True, check=True
    )
    return done.stdout


def test_production_writes_one_json_object_per_event_with_the_logger_name_and_the_context():
    lines = [json.loads(line) for line in run(ENV="prod").splitlines()]

    created = lines[0]
    assert created["event"] == "user_created"
    assert created["user_id"] == 7
    assert created["request_id"] == "r-1"
    assert created["logger"] == "app.users"
    assert created["level"] == "info"
    assert created["timestamp"].endswith("Z")


def test_production_writes_the_traceback_as_data():
    failed = json.loads(run(ENV="production").splitlines()[1])

    assert failed["event"] == "user_failed"
    assert failed["exception"][0]["exc_type"] == "ZeroDivisionError"


def test_development_writes_a_console_line_and_a_formatted_traceback():
    text = re.sub(r"\x1b\[[0-9;]*m", "", run(ENV="dev"))

    assert "user_created" in text
    assert "user_id=7" in text
    assert "ZeroDivisionError" in text
    assert not text.lstrip().startswith("{")


def test_the_level_comes_from_log_level():
    assert "cache_miss" not in run(ENV="prod")
    assert "cache_miss" in run(ENV="prod", LOG_LEVEL="debug")
    assert "user_created" not in run(ENV="prod", LOG_LEVEL="ERROR")
