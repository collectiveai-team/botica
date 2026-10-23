import numpy as t
import typing


def halves(values: list[float]) -> object:
    return t.cast[float](values)


def name(value: object) -> str:
    return typing.get_origin(value).__name__
