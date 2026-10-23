#[cfg(test)]
mod tests {
    use crate::Cart;

    #[test]
    fn adds() {
        let mut cart = Cart::new();
        cart.add(1);
        let items = cart.items().iter().collect::<Vec<_>>();
        assert_eq!(items.len(), 1);
        assert_eq!(cart.total().unwrap(), 1);
    }
}
