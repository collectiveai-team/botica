import sys
import unittest


class OrderTest(unittest.TestCase):
    @unittest.skip("the export moves to the new API in #120")
    def test_a_skip(self): ...

    @unittest.skipIf(sys.platform == "win32", "no fork on Windows")
    def test_a_skipif(self): ...

    @unittest.expectedFailure
    def test_a_known_bug(self):
        self.assertEqual(round(2.5), 3)

    def test_a_skip_in_the_body(self):
        self.skipTest("the data set is not downloaded")
