from asyncio import get_event_loop


def schedule(callback) -> None:
    get_event_loop().call_soon(callback)
