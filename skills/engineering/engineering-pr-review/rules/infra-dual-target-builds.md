---
title: Edge builds target both x86/CUDA and Jetson with separate Dockerfiles
section: infra
scope: general
applies-to: edge
status: current
tags: docker, jetson, edge, arm64, cuda, infra
---

## Edge builds target both x86/CUDA and Jetson with separate Dockerfiles

Edge AI repos ship two Dockerfiles per service: one for x86 (CUDA base) and one for
Jetson (ARM64, `dustynv` base images with TensorRT). Compose profiles (`x86_64` / `arm64`)
gate which services run on each target. Makefile targets mirror this split
(`run-demo-local` vs `run-demo-jetson`).

**Prefer (separate Dockerfiles; compose profiles; Makefile aliases):**

```dockerfile
# docker-images/asr-stream-service/Dockerfile (x86)
FROM nvcr.io/nvidia/cuda:11.7.1-runtime-ubuntu22.04
RUN apt-get install -y portaudio19-dev libsndfile1 ffmpeg

# docker-images/asr-stream-service/Dockerfile.jetson (ARM64)
FROM dustynv/whisper_trt:r36.3.0
```

```yaml
# docker-compose.yml
services:
  ollama-local:
    image: ollama/ollama          # x86
    profiles: [x86_64]

  ollamaJetson:
    image: dustynv/ollama         # ARM64
    profiles: [arm64]
```

```makefile
run-demo-local:   # x86 path
	$(MAKE) start-ollama && $(MAKE) build-asr-ubuntu && $(MAKE) start-asr-ubuntu

run-demo-jetson:  # Jetson path
	$(MAKE) start-ollama-jetson && $(MAKE) build-asr-jetson && $(MAKE) start-asr-jetson
```

**Avoid:**

```dockerfile
# Don't use a single Dockerfile with runtime platform detection — maintenance is harder
ARG TARGETARCH
RUN if [ "$TARGETARCH" = "arm64" ]; then pip install jetson-stuff; fi
```

Reference: see `asr-models` for the model-selection env var pattern used in both targets.
