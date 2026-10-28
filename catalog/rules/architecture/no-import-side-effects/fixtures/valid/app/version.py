from functools import cache
from pathlib import Path


@cache
def version() -> str:
    return Path(__file__).with_name("VERSION").read_text().strip()
