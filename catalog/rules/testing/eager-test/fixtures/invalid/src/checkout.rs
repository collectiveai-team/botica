#[cfg(test)]
mod tests {
    use crate::{checkout, send_receipt, Cart};

    #[test]
    fn checkout_flow() {
        let mut cart = Cart::new();
        cart.add(1);
        cart.remove(1);
        cart.apply_coupon("A");
        cart.pay();
        checkout(&mut cart);
        send_receipt(&cart);
        assert_eq!(cart.total(), 0);
    }
}
