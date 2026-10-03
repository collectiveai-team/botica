#!/usr/bin/env python3
"""Minimal client for NVIDIA NIM's OpenAI-compatible chat completions API.

Used by grader.py and comparator.py as an LLM judge (glm-5.2), independent
of the OpenCode/Promptfoo executor path used to drive the skill-under-test.
Reads the API key from OpenCode's own auth store so no secret ever needs to
be exported into the shell environment or printed.
"""

import json
import os
import urllib.error
import urllib.request

NIM_BASE_URL = "https://integrate.api.nvidia.com/v1"
DEFAULT_MODEL = "z-ai/glm-5.2"
_AUTH_PATH = os.path.expanduser("~/.local/share/opencode/auth.json")


def _load_key() -> str:
    with open(_AUTH_PATH, encoding="utf-8") as fh:
        auth = json.load(fh)
    return auth["nvidia"]["key"]


def chat(
    messages: list[dict],
    model: str = DEFAULT_MODEL,
    temperature: float = 0.0,
    max_tokens: int = 2000,
    timeout: int = 180,
) -> str:
    """Send a chat completion request, return the assistant text content."""
    key = _load_key()
    body = json.dumps(
        {
            "model": model,
            "messages": messages,
            "temperature": temperature,
            "max_tokens": max_tokens,
            "stream": False,
        }
    ).encode()
    req = urllib.request.Request(
        NIM_BASE_URL + "/chat/completions",
        data=body,
        headers={
            "Authorization": "Bearer " + key,
            "Content-Type": "application/json",
            "Accept": "application/json",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            data = json.loads(resp.read())
    except urllib.error.HTTPError as exc:
        raise RuntimeError(
            f"NIM chat call failed ({exc.code}): {exc.read().decode()[:400]}"
        ) from exc
    return data["choices"][0]["message"].get("content", "")


def chat_json(messages: list[dict], model: str = DEFAULT_MODEL, **kwargs) -> dict:
    """Like chat(), but parses the response as JSON (stripping ``` fences)."""
    text = chat(messages, model=model, **kwargs)
    stripped = text.strip()
    if stripped.startswith("```"):
        stripped = stripped.split("\n", 1)[1] if "\n" in stripped else stripped
        if stripped.endswith("```"):
            stripped = stripped[:-3]
        if stripped.startswith("json"):
            stripped = stripped[4:]
    try:
        return json.loads(stripped)
    except json.JSONDecodeError as exc:
        raise RuntimeError(f"Judge did not return valid JSON: {text[:500]}") from exc
