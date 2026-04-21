//! Numeric-literal parsing for the parameter-mapping language.
//!
//! Split from [`super::parser`] so the recursive-descent engine is not
//! cluttered by the dot-decimal / underscore-separator handling. All code
//! here is float-free and operates in Q32.32.

use beast_core::Q3232;

/// Parse a dot-decimal ASCII literal (already stripped of `_`) into a
/// [`Q3232`]. Returns `None` when neither the integer nor the combined
/// rational representation fit, or when the string is malformed for either
/// path.
pub(super) fn parse_q3232_literal(cleaned: &str) -> Option<Q3232> {
    if let Some(dot) = cleaned.find('.') {
        let int_part = &cleaned[..dot];
        let frac_part = &cleaned[dot + 1..];
        if frac_part.is_empty() {
            return None;
        }
        // Parse as (numerator / 10^frac_len) in Q32.32 arithmetic so we
        // avoid floats. Both pieces must fit in `i64`.
        let int_val: i64 = int_part.parse().ok()?;
        let frac_val: i64 = frac_part.parse().ok()?;
        let scale: i64 = 10_i64.checked_pow(u32::try_from(frac_part.len()).ok()?)?;
        // int_val + frac_val / scale, in Q32.32.
        let int_q = Q3232::from_num(int_val);
        let frac_q = Q3232::from_num(frac_val).saturating_div(Q3232::from_num(scale));
        Some(int_q.saturating_add(frac_q))
    } else {
        let int_val: i64 = cleaned.parse().ok()?;
        Some(Q3232::from_num(int_val))
    }
}

/// Strip underscore digit-separators from a literal slice.
///
/// `fixed`'s `FromStr` rejects `_`, so we normalise the byte slice to an
/// ASCII-only [`String`] before handing it to [`parse_q3232_literal`].
pub(super) fn strip_underscores(raw: &[u8]) -> String {
    raw.iter()
        .filter(|b| **b != b'_')
        .map(|b| *b as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_integer_only() {
        assert_eq!(parse_q3232_literal("42"), Some(Q3232::from_num(42_i32)));
    }

    #[test]
    fn parses_fractional() {
        assert_eq!(parse_q3232_literal("0.5"), Some(Q3232::from_num(0.5_f64)));
    }

    #[test]
    fn rejects_trailing_dot() {
        assert_eq!(parse_q3232_literal("3."), None);
    }

    #[test]
    fn strips_underscore_separators() {
        assert_eq!(strip_underscores(b"1_000_000"), "1000000");
        assert_eq!(strip_underscores(b"0.1_2"), "0.12");
    }
}
