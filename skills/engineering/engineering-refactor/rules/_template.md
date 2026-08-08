---
title: Short Imperative Rule Title
section: python-toolchain        # must match a section id in _sections.md
scope: general                   # general | specific
applies-to: all                  # all | comma list of stack tags: python, fastapi, prefect, asr, llm, nextjs, k8s, edge
status: current                  # current (the standard) | direction (target existing code should head toward)
tags: tag1, tag2
---

## Short Imperative Rule Title

One or two sentences: the rule and *why* it exists. Keep it tight. Rules ship to external
users — they must be self-contained and generic. Do NOT add an "Evidence:" section citing
internal repo files; those won't exist for users and only waste context tokens.

**Prefer:**

```python
# the convention we want
```

**Avoid:**

```python
# the anti-pattern, with a word on why
```

**Direction (optional):** if `status: current`, note where existing code should head when it
differs from today's practice.

Reference: cross-reference sibling rule ids (e.g. `arch-deletion-test`) — those files ship with
the skill — and external public sources only (docs, attributions). No internal links or paths.
