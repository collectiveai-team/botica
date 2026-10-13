---
title: Use Silero or WebRTC VAD with env-tunable thresholds; mock the internet download in tests
section: asr
scope: specific
applies-to: asr
status: current
tags: asr, vad, silero, webrtcvad, torch, gpu, env
---

## Use Silero or WebRTC VAD with env-tunable thresholds; mock the internet download in tests

Voice Activity Detection uses one of two backends: `SileroVAD` (neural, GPU-aware) or
`WebRTCVAD` (algorithmic, frame-size constrained). All timing and sensitivity thresholds
are read from environment variables so production tuning does not require a code change.

**Silero (preferred for accuracy):**

```python
# streaming/silero_vad_model.py
self.model, _ = torch.hub.load(
    repo_or_dir="snakers4/silero-vad", model="silero_vad", force_reload=False
)
self.model = self.model.to("cuda") if torch.cuda.is_available() else self.model

# intensity 0-3 maps to threshold 0.8-0.5 (higher = more aggressive)
threshold_map = {0: 0.8, 1: 0.7, 2: 0.6, 3: 0.5}
```

**WebRTC (lighter, CPU-only):**

```python
# streaming/webrtc_vad_model.py — frames must be >= 320 bytes; pad if shorter
if len(buffer_frame) < 320:
    buffer_frame += b"\0" * (320 - len(buffer_frame))
is_speech = self.vad.is_speech(buffer_frame, self.original_sr)
```

**Key env vars (read by SpeechFrameDetector classes in stream_manager.py):**

```bash
N_FRAMES_WITHOUT_MIN_PAUSE_THRESHOLD=150   # min frames before a pause can trigger
N_FRAMES_WITHOUT_MAX_PAUSE_THRESHOLD=450   # hard cutoff — force utterance boundary
CONSECUTIVE_NO_SPEECH_LOWER_THRESHOLD=1
CONSECUTIVE_NO_SPEECH_UPPER_THRESHOLD=70   # (10 for WebRTC variant)
VAD_LEVEL=2                                # Silero intensity (0-3), read in CLI
```

**Test gap:** `torch.hub.load("snakers4/silero-vad", ...)` downloads from the internet on
first run. Any test that constructs `SileroVAD` without mocking this call will make a
network request (and fail in CI without connectivity). Mock `torch.hub.load` at the test
boundary. Cross-ref `test-coverage-gap`.

Reference: `asr-streaming-interfaces` for `VADModelInterface`; `asr-threading-bridge` for
how VAD runs in its own thread.
