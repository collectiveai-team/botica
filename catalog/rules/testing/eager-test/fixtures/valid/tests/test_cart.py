import pytest

from shop import Cart


def test_add():
    cart = Cart()
    cart.add(1)
    cart.add(2)
    assert len(cart.items()) == 2
    assert sorted(cart.items()) == [1, 2]
    assert cart.total() == pytest.approx(3)


class TestCart:
    def test_remove(self):
        cart = Cart()
        cart.add(1)
        cart.remove(1)
        with pytest.raises(KeyError):
            cart.remove(1)
