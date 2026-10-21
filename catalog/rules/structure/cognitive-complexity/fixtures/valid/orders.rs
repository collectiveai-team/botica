impl Order {
    pub fn total(&self) -> u32 {
        self.items.iter().filter(|item| item.in_stock).map(|item| item.price).sum()
    }
}
