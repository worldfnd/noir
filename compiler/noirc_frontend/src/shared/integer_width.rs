use super::Signedness;

/// The widest integer type: `u65536` and `i65536` are the last legal widths.
pub const MAX_INTEGER_WIDTH: u32 = 65536;

/// The rule in words, for diagnostics that name a width the language does not have.
pub const LEGAL_INTEGER_WIDTHS: &str = "8, 16, and every even width from 32 to 65536";

/// Whether an integer type of `bits` bits exists, for either signedness: `8`, `16`, and every
/// even width from `32` through [`MAX_INTEGER_WIDTH`]. `u1` and `i1` are spelled `bool`.
pub fn is_legal_integer_width(bits: u32) -> bool {
    matches!(bits, 8 | 16) || ((32..=MAX_INTEGER_WIDTH).contains(&bits) && bits % 2 == 0)
}

/// Reads a name of the form `u<digits>` or `i<digits>` as a signedness and a width, without
/// asking whether that width is legal. The digits must be plain decimal with no leading zero,
/// so `u0`, `u007` and `u8_` are not integer type names, nor is a width too large for a `u32`.
pub fn parse_integer_type_name(name: &str) -> Option<(Signedness, u32)> {
    let (signedness, digits) = match name.split_at_checked(1)? {
        ("u", digits) => (Signedness::Unsigned, digits),
        ("i", digits) => (Signedness::Signed, digits),
        _ => return None,
    };
    if digits.is_empty()
        || digits.starts_with('0')
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let bits = digits.parse::<u32>().ok()?;
    Some((signedness, bits))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legal_widths_are_8_16_and_even_widths_from_32_to_65536() {
        for bits in [8, 16, 32, 34, 64, 128, 130, 256, 65534, 65536] {
            assert!(is_legal_integer_width(bits), "{bits}");
        }
        for bits in [0, 1, 2, 4, 7, 9, 10, 12, 24, 31, 33, 129, 65535, 65537, 65538, u32::MAX] {
            assert!(!is_legal_integer_width(bits), "{bits}");
        }
    }

    #[test]
    fn integer_type_names_are_a_signedness_letter_and_plain_decimal_digits() {
        assert_eq!(parse_integer_type_name("u8"), Some((Signedness::Unsigned, 8)));
        assert_eq!(parse_integer_type_name("i128"), Some((Signedness::Signed, 128)));
        assert_eq!(parse_integer_type_name("u10"), Some((Signedness::Unsigned, 10)));
        assert_eq!(parse_integer_type_name("u65536"), Some((Signedness::Unsigned, 65536)));
        assert_eq!(parse_integer_type_name("u4294967295"), Some((Signedness::Unsigned, u32::MAX)));
        for name in ["u", "i", "u0", "u007", "u08", "u8_", "u1a", "U8", "u-8", "u4294967296", ""] {
            assert_eq!(parse_integer_type_name(name), None, "{name}");
        }
    }
}
