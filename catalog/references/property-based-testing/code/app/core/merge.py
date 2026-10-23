"""The merge of two sorted lists."""

from __future__ import annotations


def merge_sorted(left: list[int], right: list[int]) -> list[int]:
    """Return the items of two ascending lists, in one ascending list; either list may be empty."""
    merged: list[int] = []
    i = j = 0
    while i < len(left) and j < len(right):
        if left[i] <= right[j]:
            merged.append(left[i])
            i += 1
        else:
            merged.append(right[j])
            j += 1
    merged.extend(left[i:])
    merged.extend(right[j:])
    return merged
