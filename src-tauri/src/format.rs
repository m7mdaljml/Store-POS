/// Formats a money amount with exactly two decimals, dropping the fraction
/// entirely when it rounds to zero ("0" instead of "0.00"). This keeps
/// floating-point noise like `0.000000000000113686` out of audit-log details,
/// exports and other stored text.
pub fn money(v: f64) -> String {
    if !v.is_finite() {
        return "0".into();
    }
    let cents = (v * 100.0).round();
    if cents == 0.0 {
        "0".to_string()
    } else {
        format!("{:.2}", cents / 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_to_two_decimals() {
        assert_eq!(money(687.2099999999999), "687.21");
        assert_eq!(money(5.0), "5.00");
        assert_eq!(money(1234.5), "1234.50");
        assert_eq!(money(-12.345), "-12.35");
    }

    #[test]
    fn zero_and_noise_become_literal_zero() {
        assert_eq!(money(0.0), "0");
        assert_eq!(money(-0.0), "0");
        assert_eq!(money(0.000000000000113686), "0");
        assert_eq!(money(-0.004), "0");
        assert_eq!(money(f64::NAN), "0");
        assert_eq!(money(f64::INFINITY), "0");
    }
}