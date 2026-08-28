---
title: Use a single /ws endpoint with per-connection StreamManager via dependency override
section: asr
scope: specific
applies-to: asr
status: current
tags: asr, websocket, fastapi, streaming, typer
---

## Use a single /ws endpoint with per-connection StreamManager via dependency override

The ASR server exposes exactly one WebSocket route (`/ws`). Each accepted connection gets
its own fresh `StreamManager` — constructed by a factory injected via
`server.dependency_overrides[dummy_manager_init]`. Never share a StreamManager across
connections; its VAD and ASR threads are per-connection state.

**Prefer:**

```python
# server/websocket_server.py
def dummy_manager_init() -> StreamManager:
    raise NotImplementedError("This function must be overwrited")

async def websocket_endpoint(
    websocket: WebSocket,
    stream_manager=Depends(dummy_manager_init),
    chunk_size: int = 1024,
):
    ...

server = FastAPI()
server.websocket("/ws")(websocket_endpoint)
server.dependency_overrides[dummy_manager_init] = stream_manager_init  # per-connection factory
```

**Avoid:**

```python
# Constructing a single global StreamManager before the server starts
manager = StreamManager(model, vad, policy, sr)  # shared — race conditions guaranteed
```

**Wire-protocol notes:**
- Incoming audio is buffered and flushed in 1024-byte chunks (`chunk_size=1024`; twice the
  512-sample Silero window because each sample is 2 bytes / 16-bit PCM).
- End-of-utterance is signalled by a sentinel command sent as raw bytes from the client.
  Both sender and receiver must agree on the exact byte sequence — do not alter it
  unilaterally.

Reference: see `asr-threading-bridge` for how the handler offloads work to sync threads,
and `pylayout-cli-typer-rich` for the Typer-wraps-FastAPI CLI entrypoint shape.
