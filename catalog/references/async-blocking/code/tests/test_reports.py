"""The reports, read inside an event loop with blockbuster active."""

from __future__ import annotations

import asyncio
from typing import TYPE_CHECKING

import pytest
from blockbuster import BlockingError

from app.core.reports import get_report, list_reports, load_report

if TYPE_CHECKING:
    from pathlib import Path


def test_the_async_functions_read_the_files_in_a_thread(tmp_path: Path):
    (tmp_path / "r1.md").write_text("# First", encoding="utf-8")
    (tmp_path / "r2.md").write_text("# Second", encoding="utf-8")

    async def read() -> tuple[list[str], str]:
        return await list_reports(tmp_path), await get_report(tmp_path, "r1")

    assert asyncio.run(read()) == (["r1", "r2"], "# First")


def test_a_blocking_call_of_the_package_inside_the_event_loop_fails(tmp_path: Path):
    (tmp_path / "r1.md").write_text("# First", encoding="utf-8")

    async def read() -> str:
        return load_report(tmp_path, "r1")

    with pytest.raises(BlockingError):
        asyncio.run(read())


def test_a_blocking_call_of_the_test_itself_does_not_fail(tmp_path: Path):
    async def write_then_list() -> list[str]:
        (tmp_path / "r1.md").write_text("# First", encoding="utf-8")
        return await list_reports(tmp_path)

    assert asyncio.run(write_then_list()) == ["r1"]


def test_a_blocking_call_outside_an_event_loop_does_not_fail(tmp_path: Path):
    (tmp_path / "r1.md").write_text("# First", encoding="utf-8")

    assert load_report(tmp_path, "r1") == "# First"
