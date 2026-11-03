pub use monopoly_proto::Money;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_money_math_overflow_and_formatting() {
        let m1 = Money::from_vnd(2_000_000);
        let m2 = Money::from_vnd(-500_000);
        assert_eq!(m1.to_string(), "2.000.000 đ");
        assert_eq!(m2.to_string(), "-500.000 đ");
        assert_eq!(m1.as_vnd(), 2_000_000);
        assert_eq!(m2.as_vnd(), -500_000);

        let sum = m1 + Money::from_vnd(1_000_000);
        assert_eq!(sum.to_string(), "3.000.000 đ");

        let diff = m1 - Money::from_vnd(3_000_000);
        assert_eq!(diff.to_string(), "-1.000.000 đ");
    }
}
