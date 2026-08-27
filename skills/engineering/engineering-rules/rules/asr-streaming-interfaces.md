---
title: Define ASR/VAD/chunking contracts as ABCs in meta/
section: asr
scope: specific
applies-to: asr
status: current
tags: asr, interfaces, abc, meta, streaming
---

## Define ASR/VAD/chunking contracts as ABCs in meta/

All ASR, VAD, and chunk-policy contracts live in `meta/streaming_interfaces.py` as
abstract base classes. Concrete model and VAD implementations subclass these interfaces —
they never reach into each other directly. This is the seam that makes models and VADs
swappable at runtime.

**Prefer:**

```python
# meta/streaming_interfaces.py
class ASRStreamingInterface:
    @abstractmethod
    def frames_to_text(self, frames: np.ndarray, previous_transcript: str, **kwargs): ...

class VADModelInterface:
    @abstractmethod
    def user_is_speaking(self, buffer_frame: bytes): ...
    @abstractmethod
    def user_is_speaking_with_proba(self, buffer_frame: bytes) -> float: ...
    @abstractmethod
    def reset_states(self): ...

class ChunkPolicyInterface:
    @abstractmethod
    def process_audio_frames(self, audio_frames): ...
    @abstractmethod
    def consume_final_prediction(self): ...
    # … other lifecycle methods

class PolicyStates(Enum):
    SKIP = 0
    CONSUME_PARTIAL_PREDS = 1
    CONSUME_FRAMES = 2
    CONSUME_FINAL_PREDICTION = 3
```

**Avoid:**

```python
# Importing SileroVAD directly inside StreamManager — couples the orchestrator to a backend
from streaming.silero_vad_model import SileroVAD
self.vad = SileroVAD(...)  # breaks swappability; type should be VADModelInterface
```

`StreamManager.__init__` should type all three constructor args (`model`, `vad`, `policy`)
as these interfaces. `VADModelInterface` may be defined with only `user_is_speaking` in
simpler variants; add `user_is_speaking_with_proba` and `reset_states` when richer VAD
control is needed.

Reference: `pylayout-meta-slice` (meta/ holds ABCs), `pylayout-adapters-slices`
(concrete impls live in sibling slices). Cross-ref `asr-models` and `asr-vad` for concrete
subclasses.
