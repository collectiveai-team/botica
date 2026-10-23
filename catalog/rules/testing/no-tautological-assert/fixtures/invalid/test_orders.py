import unittest


def test_total():
    assert True


def test_same():
    total = 3
    assert total == total


def test_same_message():
    total = 3
    assert total is total, "the total"


class TestOrder(unittest.TestCase):
    def test_flag(self):
        self.assertTrue(True)

    def test_equal(self):
        total = 3
        self.assertEqual(total, total)
