import functools


@functools.lru_cache(maxsize=int(open("size.txt").read()))
def lookup(key: str) -> str:
    return key
