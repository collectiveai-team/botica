def payload() -> dict:
    return load()


def counts() -> dict[str, int]:
    return load()


async def fetch() -> dict:
    return await load()
