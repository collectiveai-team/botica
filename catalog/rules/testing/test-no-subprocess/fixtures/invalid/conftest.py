import subprocess

import pytest


@pytest.fixture
def database():
    server = subprocess.Popen(["postgres", "-D", "data"])
    yield server
    server.terminate()
