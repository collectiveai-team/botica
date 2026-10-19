from dataclasses import dataclass

Names = list[str]
limit = 3


@dataclass
class Point:
    x: int
    y: int
