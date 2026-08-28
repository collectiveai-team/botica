---
title: Keep ASR backends behind ASRStreamingInterface; select at runtime via env or CLI flag
section: asr
scope: specific
applies-to: asr
status: current
tags: asr, whisper, nemo, deepgram, jetson, librosa, model-selection
---

## Keep ASR backends behind ASRStreamingInterface; select at runtime via env or CLI flag

Multiple ASR backends are available. All implement `ASRStreamingInterface` so
`StreamManager` never needs to know which one is running. The active backend is chosen at
server startup — never hardcoded inside `StreamManager`.

**Backends:**

| Backend | Package | Notes |
|---|---|---|
| `WhisperForStreaming` | `openai-whisper` | Loads `base.en` by default; pass `language=` to override |
| `ConformerCTCForStreaming` | `nemo` + `pyctcdecode` | NeMo `EncDecCTCModelBPE`; local path or Hugging Face pretrained |
| `DeepgramStreamHandler` | `deepgram-sdk` | Event-driven; may not subclass `ASRStreamingInterface` directly |
| `WhisperForStreaming` (TRT) | `whisper_trt` | Jetson-specific; `load_trt_model("base.en")` at module import time |

**Model selection:**

```python
# server/websocket_server.py
MODELS = {"whisper": WhisperForStreaming, "custom": load_custom_model}

def load_custom_model(model_name) -> ASRStreamingInterface:
    custom_model_path = os.getenv("CUSTOM_MODEL_PATH")   # e.g. "mypackage.models.MyModel"
    module_path, class_name = custom_model_path.rsplit(".", 1)
    CustomModelClass = getattr(import_module(module_path), class_name)
    assert issubclass(CustomModelClass, ASRStreamingInterface)
    return CustomModelClass(model_name=model_name)
```

**Sample-rate conversion:** when the client SR differs from the model SR, use
`librosa.resample`:

```python
# streaming/utils/audio_processing.py
librosa.resample(mono_data, orig_sr=native_sr, target_sr=source_sr)
```

**Prefer:**

```python
# Pass the model in as ASRStreamingInterface — StreamManager stays decoupled
manager = StreamManager(model=WhisperForStreaming("base.en"), vad=..., policy=..., sr=16000)
```

**Avoid:**

```python
# Branching on model type inside StreamManager
if isinstance(self.asr_model, WhisperForStreaming):
    ...  # breaks the interface contract
```

Reference: `asr-streaming-interfaces` for the ABC contract; `infra-dual-target-builds` for
the x86/Jetson dual-Dockerfile build that selects the TRT backend.
