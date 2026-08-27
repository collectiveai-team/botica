---
title: Split functions over ~80 lines or with deep nesting
section: spaghetti
scope: general
applies-to: all
status: direction
tags: function-size, nesting, readability, heuristic
---

## Split functions over ~80 lines or with deep nesting

A function longer than ~80 lines or with more than two levels of nested conditionals is
doing more than one thing. Split by domain concept or adapter seam — not by arbitrary
line buckets. The ~80-line mark is a smell trigger, not a hard rule: a 90-line function
with a single clean loop may be fine; a 40-line function with four nested `if/for/try`
blocks may not be.

**Prefer (split by domain step):**

```python
def parse_menu(raw_text: str) -> Menu:
    lines = _split_into_sections(raw_text)
    items = [_parse_item(line) for line in lines if _is_item(line)]
    return Menu(items=items)

def _parse_item(line: str) -> MenuItem: ...
def _is_item(line: str) -> bool: ...
def _split_into_sections(raw_text: str) -> list[str]: ...
```

**Avoid (one function doing parsing + validation + transformation + formatting):**

```python
def text_to_json(text):  # 45 lines, 3 levels of nesting
    json_output = {"_id": "some-id", "items": []}
    lines = text.split("\n")
    current_item = None
    for line in lines:
        line = line.strip()
        if line.startswith("- Item:"):
            for part in line.split(","):
                if "Quantity:" in part:
                    quantity = int(part.split(":")[1].strip())
                    # ... continues nesting
```

**Signals that a function needs splitting:**

- Scrolling is required to read it top to bottom.
- The function comment describes two or more distinct steps with "and" or "then".
- A local variable is only used in one block that could be its own function.
- Deep `if/for/try` nesting makes the happy path hard to follow.
- The function is hard to name without using "and" or "also".

Reference: see `spaghetti-large-file-thresholds` for the file-level complement and
`arch-deep-modules` for the leverage framing — a well-named extracted function is itself
a small deep module.
