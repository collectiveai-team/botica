"""A temporary Prefect server for the tests, and the logging that the workers of the topic use.

Prefect reads its logging settings when it is imported, so they are set here, before any test
module imports it.
"""

from __future__ import annotations

import os
import tempfile
from typing import TYPE_CHECKING

os.environ["PREFECT_HOME"] = tempfile.mkdtemp(prefix="prefect-home-")
os.environ["PREFECT_LOGGING_EXTRA_LOGGERS"] = "app"
os.environ["PREFECT_LOGGING_ROOT_LEVEL"] = "INFO"
os.environ["ENV"] = "prod"

import pytest
from prefect.testing.utilities import prefect_test_harness

if TYPE_CHECKING:
    from collections.abc import Iterator

NETWORK = ("integration", "e2e")
OWN_SOCKET_MARKERS = ("enable_socket", "disable_socket", "allow_hosts")


def pytest_collection_modifyitems(items: list[pytest.Item]) -> None:
    # A test that names a marker of pytest-socket keeps it: `enable_socket` would override it.
    for item in items:
        needs_network = any(item.get_closest_marker(name) for name in NETWORK)
        has_own_marker = any(item.get_closest_marker(name) for name in OWN_SOCKET_MARKERS)
        if needs_network and not has_own_marker:
            item.add_marker(pytest.mark.enable_socket)


@pytest.fixture(autouse=True, scope="session")
def prefect_server() -> Iterator[None]:
    with prefect_test_harness():
        yield
