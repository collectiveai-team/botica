"""The reports of the service: Markdown files in a folder, read without blocking the event loop."""

from __future__ import annotations

import asyncio
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from pathlib import Path


def load_report(folder: Path, report_id: str) -> str:
    """Read the report from its file: a blocking call, which async code runs in a thread."""
    return (folder / f"{report_id}.md").read_text(encoding="utf-8")


def report_ids(folder: Path) -> list[str]:
    """Return the ids of the reports of the folder, sorted: a blocking call too."""
    return sorted(path.stem for path in folder.glob("*.md"))


async def get_report(folder: Path, report_id: str) -> str:
    """Return the text of the report."""
    return await asyncio.to_thread(load_report, folder, report_id)


async def list_reports(folder: Path) -> list[str]:
    """Return the ids of the reports."""
    return await asyncio.to_thread(report_ids, folder)
