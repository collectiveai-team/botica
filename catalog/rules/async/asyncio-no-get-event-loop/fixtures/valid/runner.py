import asyncio


async def fetch() -> int:
    return 1


def migrate() -> None:
    pass


def run() -> None:
    with asyncio.Runner() as runner:
        runner.run(fetch())
        migrate()
        runner.run(fetch())
