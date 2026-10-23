"""The merge of sorted lists, checked against `sorted` for any two sorted lists."""

from __future__ import annotations

from hypothesis import given
from hypothesis import strategies as st

from app.core.merge import merge_sorted

sorted_lists = st.lists(st.integers()).map(sorted)


@given(sorted_lists, sorted_lists)
def test_the_merge_has_every_item_of_both_lists_in_order(left: list[int], right: list[int]):
    assert merge_sorted(left, right) == sorted(left + right)


def test_the_merge_keeps_equal_items_from_both_lists():
    assert merge_sorted([1, 2], [2, 3]) == [1, 2, 2, 3]
