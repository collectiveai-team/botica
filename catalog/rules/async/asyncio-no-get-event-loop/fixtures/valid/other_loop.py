from app.loops import get_event_loop


def schedule(policy, callback) -> None:
    policy.get_event_loop().call_soon(callback)
    get_event_loop().call_soon(callback)
