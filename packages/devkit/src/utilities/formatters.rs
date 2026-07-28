/// Format fee value in stroops with comma separators
pub fn format_stroops(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect::<String>() + " str"
}

/// Format fee value as XLM (1 XLM = 10,000,000 stroops)
pub fn format_xlm(n: u64) -> String {
    let xlm = n as f64 / 10_000_000.0;
    format!("{:.7} XLM", xlm)
}

/// Auto-format: show stroops below 1M, XLM above
pub fn format_fee_short(n: u64) -> String {
    if n < 1_000_000 {
        format_stroops(n)
    } else {
        format_xlm(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_stroops() {
        assert_eq!(format_stroops(0), "0 str");
        assert_eq!(format_stroops(100), "100 str");
        assert_eq!(format_stroops(3849), "3,849 str");
        assert_eq!(format_stroops(1000000), "1,000,000 str");
    }

    #[test]
    fn test_format_xlm() {
        assert_eq!(format_xlm(0), "0.0000000 XLM");
        assert_eq!(format_xlm(10_000_000), "1.0000000 XLM");
        assert_eq!(format_xlm(3849), "0.0003849 XLM");
    }

    #[test]
    fn test_format_fee_short() {
        assert_eq!(format_fee_short(3849), "3,849 str");
        assert_eq!(format_fee_short(10_000_000), "1.0000000 XLM");
    }
}
