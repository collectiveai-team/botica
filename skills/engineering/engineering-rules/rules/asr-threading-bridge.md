---
title: Bridge the async WebSocket handler to sync VAD/ASR via threads and queue.Queue
section: asr
scope: specific
applies-to: asr
status: current
tags: asr, threading, asyncio, concurrency, queue, stream-manager
---

## Bridge the async WebSocket handler to sync VAD/ASR via threads and queue.Queue

The concurrency model is a deliberate bridge, not asyncio throughout. The async WebSocket
handler feeds raw bytes into a sync `queue.Queue`; `StreamManager` runs a VAD thread and
an ASR thread that communicate over a second `queue.Queue`; a third thread (`start_new_thread`)
drains the ASR output queue and deposits into an `asyncio.Queue` so the async handler can
send results back to the client.

```
async handler
    │ await websocket.receive_bytes()
    ▼
in_queue (queue.Queue)          ← sync, blocking
    │ stream_func = lambda: in_queue.get()
    ▼
StreamManager.vad_process (threading.Thread)
    │ frames or string sentinels → asr_input_queue (queue.Queue)
    ▼
StreamManager.asr_process (threading.Thread)
    │ transcripts or "close" → asr_output_queue (queue.Queue)
    ▼
read_outqueue (_thread.start_new_thread)   ← bridges sync→async
    │ out_queue.put_nowait(response)
    ▼
out_queue (asyncio.Queue)
    │ await / get_nowait
    ▼
async handler → websocket.send_json(...)
```

**String sentinels in use (stringly-typed — a known caveat):**

| Sentinel | Direction | Meaning |
|---|---|---|
| `"pause_on_speech"` | VAD → ASR thread | Flush current utterance |
| `"close"` | handler → VAD; VAD → ASR; ASR → output | Tear down connection |
| a sentinel command (bytes) | client → handler | Force-flush last utterance (wire protocol) |

**Caveats to know:**
1. Sentinels are plain strings mixed with byte payloads in the same queue. A typed message
   class (`@dataclass` or `Union[bytes, Literal["pause_on_speech","close"]]`) would be safer.
2. Thread lifecycle is per-connection. `StreamManager.stop()` must be called in the `finally`
   block of every handler or threads leak. Call `stream_manager.stop()` then
   `in_queue.put("close")` to drain cleanly.
3. `_thread.start_new_thread` (the low-level C thread) is used for the sync→async bridge
   because it does not need a running event loop; do not replace it with `asyncio.to_thread`
   — that requires the event loop to be running and is called from within async context only.

Reference: `asr-websocket-server` for the endpoint shape; `asr-vad` for what runs inside
the VAD thread.
