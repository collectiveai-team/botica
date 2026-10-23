import subprocess
from typing import Protocol


class Remote(Protocol):
    def push(self, tag: str) -> None: ...


class GitRemote:
    def push(self, tag: str) -> None:
        subprocess.run(["git", "push", "origin", tag], check=True)


class Releases:
    def __init__(self, remote: Remote) -> None:
        self.remote = remote

    def publish(self, tag: str) -> None:
        self.remote.push(tag)
