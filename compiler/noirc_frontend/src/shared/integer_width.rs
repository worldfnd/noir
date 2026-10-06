use super::Signedness;

/// The widest integer type: `u16384` and `i16384` are the last legal widths. The cap is the one
/// Mavros lowers (its `MAX_SUPPORTED_INT_BITS`), so no width the front half admits is one the
/// consumer of the monomorphized output refuses on principle.
pub const MAX_INTEGER_WIDTH: u32 = 16384;

/// Whether an integer type of `bits` bits exists, for either signedness: every width from `2`
/// through [`MAX_INTEGER_WIDTH`], odd or even. Width 1 is refused in every spelling rather than
/// aliased to `bool`; `design/field-genericity.md` records why.
pub fn is_legal_integer_width(bits: u32) -> bool {
    (2..=MAX_INTEGER_WIDTH).contains(&bits)
}

/// The integer types the circuit backend lowers, in words, for diagnostics.
pub const LOWERABLE_INTEGER_TYPES: &str = "u8, u16, u32, u64, u128, i8, i16, i32 and i64";

/// Whether ACIR and Brillig have a lowering for an integer type, which is also the set an entry
/// point takes or returns. The front half and every other consumer of the monomorphized output
/// admit each width the language has inside a program. The signed set stops at 64 bits because
/// the ACIR lowering of signed comparison, and of the truncation after signed arithmetic,
/// carries one bit more than the operand.
pub fn is_lowerable_integer_width(signedness: Signedness, bits: u32) -> bool {
    match signedness {
        Signedness::Unsigned => matches!(bits, 8 | 16 | 32 | 64 | 128),
        Signedness::Signed => matches!(bits, 8 | 16 | 32 | 64),
    }
}

/// Reads a name of the form `u<digits>` or `i<digits>` as a signedness and a width, without
/// asking whether that width is legal: `u0` and `u1` parse, and the legality rule refuses them.
/// The digits must be plain decimal with no leading zero, so `u00`, `u007` and `u8_` are not
/// integer type names, nor is a width too large for a `u32`.
pub fn parse_integer_type_name(name: &str) -> Option<(Signedness, u32)> {
    let (signedness, digits) = match name.split_at_checked(1)? {
        ("u", digits) => (Signedness::Unsigned, digits),
        ("i", digits) => (Signedness::Signed, digits),
        _ => return None,
    };
    if digits.is_empty()
        || (digits.starts_with('0') && digits.len() > 1)
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
    fn legal_widths_are_every_width_from_2_to_16384() {
        for bits in [2, 3, 8, 16, 33, 34, 64, 128, 129, 16383, 16384] {
            assert!(is_legal_integer_width(bits), "{bits}");
        }
        for bits in [0, 1, 16385, u32::MAX] {
            assert!(!is_legal_integer_width(bits), "{bits}");
        }
    }

    #[test]
    fn the_lowerable_types_are_the_five_unsigned_and_four_signed_powers_of_two() {
        for bits in [8, 16, 32, 64, 128] {
            assert!(is_lowerable_integer_width(Signedness::Unsigned, bits), "u{bits}");
        }
        for bits in [8, 16, 32, 64] {
            assert!(is_lowerable_integer_width(Signedness::Signed, bits), "i{bits}");
        }
        assert!(!is_lowerable_integer_width(Signedness::Signed, 128), "i128");
        for bits in [2, 3, 24, 34, 66, 253, 256, MAX_INTEGER_WIDTH] {
            assert!(!is_lowerable_integer_width(Signedness::Unsigned, bits), "u{bits}");
            assert!(!is_lowerable_integer_width(Signedness::Signed, bits), "i{bits}");
        }
    }

    #[test]
    fn integer_type_names_are_a_signedness_letter_and_plain_decimal_digits() {
        assert_eq!(parse_integer_type_name("u8"), Some((Signedness::Unsigned, 8)));
        assert_eq!(parse_integer_type_name("i128"), Some((Signedness::Signed, 128)));
        assert_eq!(parse_integer_type_name("u10"), Some((Signedness::Unsigned, 10)));
        assert_eq!(parse_integer_type_name("u16384"), Some((Signedness::Unsigned, 16384)));
        assert_eq!(parse_integer_type_name("u4294967295"), Some((Signedness::Unsigned, u32::MAX)));
        // A width the language does not have still parses; legality is a separate question.
        assert_eq!(parse_integer_type_name("u0"), Some((Signedness::Unsigned, 0)));
        assert_eq!(parse_integer_type_name("u1"), Some((Signedness::Unsigned, 1)));
        for name in ["u", "i", "u00", "u007", "u08", "u8_", "u1a", "U8", "u-8", "u4294967296", ""] {
            assert_eq!(parse_integer_type_name(name), None, "{name}");
        }
    }
}
