"""The settings of the topic, read from the environment and `.env`, and replaced in a test."""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest
from fastapi import FastAPI
from fastapi.testclient import TestClient

from app.api.v1.routers.status import router
from app.settings import Settings, get_settings

if TYPE_CHECKING:
    from collections.abc import Iterator
    from pathlib import Path


@pytest.fixture(autouse=True)
def fresh(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> Iterator[None]:
    monkeypatch.chdir(tmp_path)
    for name in ("APP_DEBUG", "APP_LOG_LEVEL", "APP_DATABASE_URL"):
        monkeypatch.delenv(name, raising=False)
    get_settings.cache_clear()
    yield
    get_settings.cache_clear()


def test_the_prefixed_variables_are_read_whatever_their_case(monkeypatch: pytest.MonkeyPatch):
    monkeypatch.setenv("app_debug", "true")
    monkeypatch.setenv("APP_LOG_LEVEL", "DEBUG")

    settings = get_settings()

    assert settings.debug is True
    assert settings.log_level == "DEBUG"


def test_the_env_file_is_read_and_its_undeclared_entries_are_ignored(tmp_path: Path):
    (tmp_path / ".env").write_text("APP_DATABASE_URL=postgresql://db/app\nAPP_UNKNOWN=1\n")

    assert get_settings().database_url == "postgresql://db/app"


def test_the_settings_are_built_once_per_process(monkeypatch: pytest.MonkeyPatch):
    first = get_settings()
    monkeypatch.setenv("APP_DEBUG", "true")

    assert get_settings() is first
    get_settings.cache_clear()
    assert get_settings().debug is True


def test_a_test_replaces_the_settings_of_an_endpoint():
    app = FastAPI()
    app.include_router(router)
    app.dependency_overrides[get_settings] = lambda: Settings(debug=True)

    assert TestClient(app).get("/status").json() == {"debug": True}
