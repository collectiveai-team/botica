import math
import unittest


def total(prices):
    return sum(prices)


def test_total():
    assert total([1, 2]) == 3


def test_not_nan():
    value = 1.0
    assert not math.isnan(value)


def test_unreachable_branch():
    assert False, "never reached"


class TestOrder(unittest.TestCase):
    def test_equal(self):
        self.assertEqual(total([1, 2]), 3)

    def test_flag(self):
        self.assertTrue(total([1]) > 0)
