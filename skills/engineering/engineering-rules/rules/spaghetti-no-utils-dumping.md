---
title: No catch-all utils.py or helpers.py
section: spaghetti
scope: general
applies-to: all
status: direction
tags: naming, module-design, catch-all
---

## No catch-all utils.py or helpers.py

A file named `utils.py` or `helpers.py` is not a module — it is a junk drawer. It has no
coherent interface, grows without bound, and provides no leverage signal to callers. Every
function in a utils file has a stronger domain home. Name modules by domain concept or
adapter seam instead.

Only acceptable exception: the repo already has a `utils.py` convention AND the function
has no identifiable domain home. In that case, add reluctantly and document why.

**Prefer (named by domain concept):**

```python
# core/contracts/expiration.py
def get_days_until_expiration(expiration_date_str: str) -> int: ...

# llm/conversation_format.py
def format_conversation(conversation: str) -> list[dict]: ...

# llm/benchmark.py
def measure_token_generation_speed(response) -> float: ...
```

**Avoid:**

```python
# core/helpers/utils.py  ← one date helper with no domain label
def get_days_until_expiration(expiration_date_str: str) -> int: ...

# llm/utils.py  ← conversation formatting + benchmarking in one file
def format_conversation(conversation: str) -> list[dict]: ...
def measure_token_generation_speed_for_conversation(response): ...
def main(): ...
```

Reference: see `arch-deletion-test` — apply the deletion test: if you delete `utils.py`,
does complexity vanish (the functions were trivial pass-throughs) or does it spread to
callers (the functions are earning their place and belong in a named domain module)?
