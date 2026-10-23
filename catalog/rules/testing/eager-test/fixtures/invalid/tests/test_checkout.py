from shop import Cart, checkout, send_receipt


def test_checkout():
    cart = Cart()
    cart.add(1)
    cart.remove(1)
    cart.apply_coupon("A")
    checkout(cart)
    send_receipt(cart)
    assert cart.total() == 0
