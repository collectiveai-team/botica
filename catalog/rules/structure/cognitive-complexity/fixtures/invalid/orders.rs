impl Order {
    pub fn total(&self) -> u32 {
        let mut result = 0;
        if !self.items.is_empty() {
            for item in &self.items {
                if item.in_stock {
                    result += item.price;
                }
            }
        }
        result
    }
}
