from dataclasses import dataclass


@dataclass
class Point:
    x: int
    y: int


def origin() -> Point:
    return Point(x=0, y=0)


def rows() -> list[dict]:
    return []
