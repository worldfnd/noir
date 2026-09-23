//! Accept and reject vectors for the single-element ABI domain, shared by the parser and the
//! ast-interpreter input bridge.

use acvm::FieldConfig;
use num_bigint::{BigInt, BigUint};

use crate::{AbiType, Sign};

/// One spelling of a scalar input and the element it must be carried in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryVector {
    pub typ: AbiType,
    /// The value as written after `x = ` in `Prover.toml`: a TOML integer, a boolean, or a quoted
    /// string. The same text is a valid JSON value.
    pub spelling: String,
    /// The element the value is carried in, or `None` if the parser must refuse it.
    pub element: Option<BigUint>,
}

/// The boundary vectors of `field`: `p - 1`, `p` and `p + 1` for `Field`, the two
/// signednesses at the widest width the parser carries (one bit short of the modulus, which a
/// hand-written ABI can still name) and one bit wider, the 64-bit values that
/// straddle the Goldilocks modulus, a native spelling of a wide type, and the spellings each
/// scalar type refuses whatever the field.
pub fn boundary_vectors(field: FieldConfig) -> Vec<BoundaryVector> {
    let modulus = field.modulus();
    let widest = field.num_bits() - 1;
    let canonical = |element: BigUint| (element < *modulus).then_some(element);
    let mut vectors = Vec::new();
    let mut push = |typ: &AbiType, spelling: String, element: Option<BigUint>| {
        vectors.push(BoundaryVector { typ: typ.clone(), spelling, element });
    };

    let typ = AbiType::Field;
    push(&typ, "0".into(), Some(BigUint::ZERO));
    push(&typ, "1".into(), Some(BigUint::from(1u8)));
    push(&typ, "-1".into(), Some(modulus - 1u8));
    push(&typ, "\"-1\"".into(), None);
    push(&typ, format!("\"{}\"", modulus - 1u8), Some(modulus - 1u8));
    push(&typ, format!("\"0x{:x}\"", modulus - 1u8), Some(modulus - 1u8));
    push(&typ, format!("\"{modulus}\""), None);
    push(&typ, format!("\"{}\"", modulus + 1u8), None);

    let typ = AbiType::Boolean;
    for (spelling, element) in [("true", 1u8), ("false", 0), ("1", 1), ("\"0\"", 0)] {
        push(&typ, spelling.into(), Some(BigUint::from(element)));
    }
    for spelling in ["2", "\"2\"", "-1"] {
        push(&typ, spelling.into(), None);
    }

    let typ = AbiType::Integer { sign: Sign::Unsigned, width: 8 };
    push(&typ, "255".into(), Some(BigUint::from(255u8)));
    push(&typ, "\"0xff\"".into(), Some(BigUint::from(255u8)));
    for spelling in ["256", "\"0x100\"", "-1"] {
        push(&typ, spelling.into(), None);
    }

    // A native spelling reaches only i64 magnitudes, whatever the width.
    let typ = AbiType::Integer { sign: Sign::Unsigned, width: 128 };
    push(&typ, "5".into(), Some(BigUint::from(5u8)));
    push(&typ, "-1".into(), None);

    // The Goldilocks modulus is 2^64 - 2^32 + 1, so these values sit on either side of it.
    let typ = AbiType::Integer { sign: Sign::Unsigned, width: 64 };
    for value in [u64::MAX - u64::from(u32::MAX), u64::MAX - u64::from(u32::MAX) + 1, u64::MAX] {
        push(&typ, format!("\"{value}\""), canonical(BigUint::from(value)));
    }
    let typ = AbiType::Integer { sign: Sign::Signed, width: 64 };
    let two_to_the_32 = BigInt::from(1u64 << 32);
    for value in [
        BigInt::from(1),
        BigInt::from(i64::MAX),
        BigInt::from(i64::MIN),
        -two_to_the_32.clone(),
        -two_to_the_32 + 1,
        BigInt::from(-1),
    ] {
        let element = canonical(pattern(&value, 64));
        push(&typ, value.to_string(), element.clone());
        push(&typ, format!("\"{value}\""), element);
    }

    let two_to_the_widest = BigUint::from(1u8) << widest;
    let typ = AbiType::Integer { sign: Sign::Unsigned, width: widest };
    push(&typ, format!("\"{}\"", &two_to_the_widest - 1u8), Some(&two_to_the_widest - 1u8));
    push(&typ, format!("\"{two_to_the_widest}\""), None);
    let typ = AbiType::Integer { sign: Sign::Signed, width: widest };
    let half = BigInt::from(&two_to_the_widest >> 1u8);
    push(&typ, "-1".into(), Some(&two_to_the_widest - 1u8));
    push(&typ, format!("\"{}\"", -half.clone()), Some(&two_to_the_widest >> 1u8));
    push(&typ, format!("\"{}\"", &half - 1), Some((&two_to_the_widest >> 1u8) - 1u8));
    push(&typ, format!("\"{half}\""), None);
    push(&typ, format!("\"{}\"", -half - 1), None);

    // A width the compiler refuses at an entry point still reaches a hand-written ABI, where
    // the parser keeps the values below the modulus.
    let typ = AbiType::Integer { sign: Sign::Unsigned, width: widest + 1 };
    push(&typ, format!("\"{}\"", modulus - 1u8), Some(modulus - 1u8));
    push(&typ, format!("\"{modulus}\""), None);

    vectors
}

/// The fixed-width two's complement bit pattern of `value`.
fn pattern(value: &BigInt, width: u32) -> BigUint {
    let modulus = BigInt::from(1u8) << width;
    ((value % &modulus + &modulus) % &modulus).to_biguint().expect("a remainder is non-negative")
}

#[cfg(test)]
mod tests {
    use acvm::{FieldConfig, FieldId};
    use num_bigint::BigUint;
    use strum::IntoEnumIterator;

    use crate::{
        Abi, AbiParameter, AbiVisibility, encode_scalar,
        input_parser::{Format, InputValue},
        scalar::carried_fields,
    };

    use super::boundary_vectors;

    #[test]
    fn the_parser_accepts_and_refuses_exactly_the_boundary_vectors() {
        for field in carried_fields() {
            for vector in boundary_vectors(field) {
                let abi = Abi {
                    parameters: vec![AbiParameter {
                        name: "x".into(),
                        typ: vector.typ.clone(),
                        visibility: AbiVisibility::Private,
                    }],
                    return_type: None,
                    error_types: Default::default(),
                };
                for format in Format::iter() {
                    let source = match format {
                        Format::Toml => format!("x = {}", vector.spelling),
                        Format::Json => format!("{{\"x\": {}}}", vector.spelling),
                    };
                    let element = format.parse(&source, &abi, field).ok().map(|mut inputs| {
                        let serialized = format.serialize(&inputs, &abi).unwrap();
                        assert_eq!(
                            format.parse(&serialized, &abi, field).unwrap(),
                            inputs,
                            "{}, {format:?}: {vector:?}",
                            field.name()
                        );
                        let value = inputs.remove("x").expect("x is parsed");
                        assert!(matches!(value, InputValue::Field(_)));
                        encode_scalar(&value, &vector.typ, field).expect("a parsed value encodes")
                    });
                    assert_eq!(element, vector.element, "{}, {format:?}: {vector:?}", field.name());
                }
            }
        }
    }

    /// The vectors differ between fields only where the modulus decides.
    #[test]
    fn the_goldilocks_vectors_refuse_the_64_bit_patterns_at_or_above_its_modulus() {
        let goldilocks = boundary_vectors(FieldConfig::new(FieldId::Goldilocks));
        let refused_i64: Vec<_> = goldilocks
            .iter()
            .filter(|vector| {
                vector.typ == crate::AbiType::Integer { sign: crate::Sign::Signed, width: 64 }
                    && vector.element.is_none()
            })
            .map(|vector| vector.spelling.trim_matches('"').to_string())
            .collect();
        assert_eq!(refused_i64, ["-4294967295", "-4294967295", "-1", "-1"]);

        let bn254 = boundary_vectors(FieldConfig::new(FieldId::Bn254));
        let bn254_u64_elements: Vec<_> = bn254
            .iter()
            .filter(|vector| {
                vector.typ == crate::AbiType::Integer { sign: crate::Sign::Unsigned, width: 64 }
            })
            .map(|vector| vector.element.clone())
            .collect();
        assert!(bn254_u64_elements.iter().all(Option::is_some));
        assert_eq!(bn254_u64_elements.last(), Some(&Some(BigUint::from(u64::MAX))));
    }
}
