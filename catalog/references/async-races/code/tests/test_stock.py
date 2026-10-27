"""Concurrent reserves of the stock, run with `asyncio.gather` and checked on the stock after."""

from __future__ import annotations

import asyncio

from app.core.stock import Stock, UnlockedStock


async def reserve_concurrently(stock: Stock | UnlockedStock, requests: int) -> list[bool]:
    return await asyncio.gather(*(stock.reserve(1) for _ in range(requests)))


def test_concurrent_reserves_never_take_more_units_than_the_stock_has():
    stock = Stock(units=3)

    reserved = asyncio.run(reserve_concurrently(stock, requests=5))

    assert stock.units == 0
    assert reserved.count(True) == 3


def test_a_check_then_act_across_an_await_takes_more_units_than_the_stock_has():
    stock = UnlockedStock(units=3)

    reserved = asyncio.run(reserve_concurrently(stock, requests=5))

    assert stock.units == -2
    assert reserved.count(True) == 5
