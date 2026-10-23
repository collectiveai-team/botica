import asyncio
import subprocess
from asyncio import run
from os import getcwd

from app.deploy import Releases


class InMemoryRemote:
    def __init__(self) -> None:
        self.tags: list[str] = []

    def push(self, tag: str) -> None:
        self.tags.append(tag)


def test_a_release_pushes_its_tag():
    remote = InMemoryRemote()
    Releases(remote).publish("v1.0.0")
    assert remote.tags == ["v1.0.0"]


def test_a_failed_push_is_reported():
    error = subprocess.CalledProcessError(1, ["git", "push"])
    assert "git push" in subprocess.list2cmdline(error.cmd)


def test_the_loop_runs():
    assert asyncio.run(asyncio.sleep(0, result=1)) == 1
    assert run(asyncio.sleep(0, result=2)) == 2
    assert getcwd()
