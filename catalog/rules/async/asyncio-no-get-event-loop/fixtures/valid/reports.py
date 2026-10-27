import asyncio
from concurrent.futures import ProcessPoolExecutor


async def build_report(rows: list[int]) -> int:
    loop = asyncio.get_running_loop()
    with ProcessPoolExecutor() as pool:
        return await loop.run_in_executor(pool, sum, rows)
