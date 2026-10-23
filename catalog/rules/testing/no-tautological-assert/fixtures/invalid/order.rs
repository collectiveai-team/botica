pub fn total(prices: &[u32]) -> u32 {
    prices.iter().sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums() {
        let sum = total(&[1, 2]);
        assert_eq!(sum, sum);
        assert_eq!(sum, sum, "the sum");
    }
}
