package shop

import "testing"

func TestCheckout(t *testing.T) {
	cart := NewCart()
	cart.Add(1)
	cart.Remove(1)
	cart.ApplyCoupon("A")
	Checkout(cart)
	SendReceipt(cart)
	if cart.Total() != 0 {
		t.Fatal("the total is not 0")
	}
}
