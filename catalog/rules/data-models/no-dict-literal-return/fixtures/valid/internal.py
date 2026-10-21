def _payload():
    return {"ok": True}


def outer() -> int:
    def inner():
        return {"ok": True}

    return len(inner())
