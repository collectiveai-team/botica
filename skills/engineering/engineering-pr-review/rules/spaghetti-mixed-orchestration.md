---
title: Don't pile transport, domain, persistence, and formatting into one module
section: spaghetti
scope: general
applies-to: all
status: direction
tags: separation-of-concerns, orchestration, complexity
---

## Don't pile transport, domain, persistence, and formatting into one module

When a single module mixes transport (WebSocket/HTTP handling), validation, domain logic,
persistence, logging, and formatting, every concern becomes harder to change and impossible
to test in isolation. When a file already mixes concerns, find a smaller seam and extract
it — don't add another branch to the tangle.

**Prefer (one concern per module, seam at the transport boundary):**

```python
# server/websocket_server.py — transport only: accept connection, hand off to manager
async def websocket_endpoint(websocket: WebSocket, manager: StreamManager = Depends(...)):
    await manager.run(websocket)

# streaming/stream_manager.py — orchestration only: route frames to VAD + ASR
class StreamManager:
    async def run(self, websocket: WebSocket) -> None: ...

# streaming/silero_vad_model.py — VAD only: is this frame speech?
# streaming/whisper_streaming.py — ASR only: transcribe a speech segment
# formatting/transcript_formatter.py — formatting only: emit transcript lines
```

**Avoid (everything in one place):**

```python
# websocket_server.py mixes: WebSocket transport, thread management,
# VAD model selection, ASR dispatch, queue polling, and transcript formatting.
# Adding more logic here makes every concern harder to isolate and test.
async def websocket_endpoint(websocket, vad_model, asr_model, queue, formatter, ...):
    ...  # 150 lines of mixed concerns
```

Reference: see `arch-deep-modules` for seam placement principles and `spaghetti-large-file-thresholds`
for the size signals that should prompt this check.
