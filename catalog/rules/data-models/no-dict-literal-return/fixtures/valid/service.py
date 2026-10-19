from dataclasses import dataclass


@dataclass
class Point:
    x: int
    y: int


def origin():
    return Point(x=0, y=0)


def scratch():
    local = {"ok": True}
    return len(local)
