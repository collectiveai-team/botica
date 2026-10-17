"""Fixtures shared by the tests of the site."""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest
from site_references_support import Site

if TYPE_CHECKING:
    from pathlib import Path


@pytest.fixture
def site(tmp_path: Path) -> Site:
    return Site(tmp_path)
