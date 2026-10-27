"""The units of a product in stock, reserved by concurrent requests of the service."""

from __future__ import annotations

import asyncio


async def confirm_with_warehouse(units: int) -> None:
    """Stand for the call to the warehouse: an await that hands the event loop to other tasks."""
    await asyncio.sleep(0)


class UnlockedStock:
    """The bug: `reserve` checks the units, awaits, then takes them: other reserves run between."""

    def __init__(self, units: int) -> None:
        self.units = units

    async def reserve(self, units: int) -> bool:
        """Take the units and return True, or return False when the stock has fewer."""
        if self.units < units:
            return False
        await confirm_with_warehouse(units)
        self.units -= units
        return True


class Stock:
    """The fix: an `asyncio.Lock` keeps the check and the take of one reserve together."""

    def __init__(self, units: int) -> None:
        self.units = units
        self._lock = asyncio.Lock()

    async def reserve(self, units: int) -> bool:
        """Take the units and return True, or return False when the stock has fewer."""
        async with self._lock:
            if self.units < units:
                return False
            await confirm_with_warehouse(units)
            self.units -= units
            return True
