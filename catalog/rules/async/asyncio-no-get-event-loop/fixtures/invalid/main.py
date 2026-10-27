import asyncio


async def main() -> None:
    await asyncio.sleep(0)


loop = asyncio.get_event_loop()
loop.run_until_complete(main())
