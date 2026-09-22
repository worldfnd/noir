//! The scalar codec: the field element that carries each scalar ABI type, under a field chosen at
//! run time.
//!
//! A `Field` is carried as itself, an integer as its fixed-width two's complement bit pattern and
//! a boolean as `0` or `1`, each as the canonical integer in `[0, p)`. Nothing is reduced: a value
//! whose element would reach the modulus is refused. Values are held in [`InputValue::Field`],
//! whose linked field element is an exact container for every field with a modulus no larger
//! than the linked one; other fields are refused before any value is read.

use acvm::{AcirField, FieldConfig, FieldElement, FieldId};
use num_bigint::BigUint;
use thiserror::Error;

use crate::{AbiType, input_parser::InputValue};

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ScalarError {
    #[error("{0:?} is not a scalar type")]
    NotAScalar(AbiType),
    #[error("{value} is not below the {field} field modulus {}", FieldConfig::new(*field).modulus())]
    ExceedsFieldModulus { value: BigUint, field: FieldId },
    #[error("{value} is not the field element of any value of {typ:?}")]
    OutOfRange { value: BigUint, typ: AbiType },
    #[error("{}", not_carried_message(*field, *linked))]
    FieldNotCarried { field: FieldId, linked: FieldId },
}

pub(crate) fn not_carried_message(field: FieldId, linked: FieldId) -> String {
    format!(
        "values of the {field} field cannot be held in this build: its modulus exceeds that of {linked}, the field this build is linked against"
    )
}

/// The field element that carries the scalar `value` of `typ` under `field`.
pub fn encode_scalar(
    value: &InputValue,
    typ: &AbiType,
    field: FieldConfig,
) -> Result<BigUint, ScalarError> {
    ensure_carried(field)?;
    let InputValue::Field(element) = value else {
        return Err(ScalarError::NotAScalar(typ.clone()));
    };
    let pattern = pattern_of(*element);
    check_pattern(&pattern, typ, field)?;
    Ok(pattern)
}

/// The value of `typ` whose field element under `field` is `pattern`; the inverse of
/// [`encode_scalar`]. A signed integer's element is read as a bit pattern of its declared width.
pub fn decode_scalar(
    pattern: BigUint,
    typ: &AbiType,
    field: FieldConfig,
) -> Result<InputValue, ScalarError> {
    ensure_carried(field)?;
    check_pattern(&pattern, typ, field)?;
    Ok(InputValue::Field(container(&pattern)))
}

/// The field and the linked field, when the linked element cannot hold every element of `field`.
pub(crate) fn not_carried(field: FieldConfig) -> Option<(FieldId, FieldId)> {
    let linked = FieldConfig::linked();
    (field.modulus() > linked.modulus()).then(|| (field.id(), linked.id()))
}

/// Refuse a field whose elements the linked element cannot hold.
fn ensure_carried(field: FieldConfig) -> Result<(), ScalarError> {
    match not_carried(field) {
        Some((field, linked)) => Err(ScalarError::FieldNotCarried { field, linked }),
        None => Ok(()),
    }
}

/// The fields this build reads and encodes: those whose every element the linked element holds.
#[cfg(test)]
pub(crate) fn carried_fields() -> Vec<FieldConfig> {
    FieldId::ALL
        .into_iter()
        .map(FieldConfig::new)
        .filter(|field| not_carried(*field).is_none())
        .collect()
}

/// The fields this build refuses: those with an element the linked element cannot hold.
#[cfg(test)]
pub(crate) fn uncarried_fields() -> Vec<FieldConfig> {
    FieldId::ALL
        .into_iter()
        .map(FieldConfig::new)
        .filter(|field| not_carried(*field).is_some())
        .collect()
}

fn check_pattern(pattern: &BigUint, typ: &AbiType, field: FieldConfig) -> Result<(), ScalarError> {
    if !is_scalar(typ) {
        return Err(ScalarError::NotAScalar(typ.clone()));
    }
    if pattern >= field.modulus() {
        return Err(ScalarError::ExceedsFieldModulus { value: pattern.clone(), field: field.id() });
    }
    if !fits_scalar_type(pattern, typ) {
        return Err(ScalarError::OutOfRange { value: pattern.clone(), typ: typ.clone() });
    }
    Ok(())
}

fn is_scalar(typ: &AbiType) -> bool {
    matches!(typ, AbiType::Field | AbiType::Integer { .. } | AbiType::Boolean)
}

/// Whether `pattern` is the element of some value of the scalar `typ`, whatever the field: a
/// `Field` takes any element, an integer the bit patterns of its width, a boolean `0` or `1`.
fn fits_scalar_type(pattern: &BigUint, typ: &AbiType) -> bool {
    match typ {
        AbiType::Field => true,
        AbiType::Integer { width, .. } => pattern.bits() <= u64::from(*width),
        AbiType::Boolean => pattern.bits() <= 1,
        _ => false,
    }
}

/// The linked element holding `pattern`, which the caller has checked is below the modulus of a
/// carried field.
pub(crate) fn container(pattern: &BigUint) -> FieldElement {
    FieldElement::from_be_bytes_reduce(&pattern.to_bytes_be())
}

/// The canonical integer a linked element holds.
pub(crate) fn pattern_of(element: FieldElement) -> BigUint {
    BigUint::from_bytes_be(&element.to_be_bytes())
}

#[cfg(test)]
mod tests {
    use acvm::FieldConfig;
    use num_bigint::BigUint;
    use proptest::prelude::*;

    use crate::{AbiType, Sign, input_parser::InputValue};

    use super::{
        ScalarError, carried_fields, container, decode_scalar, encode_scalar, uncarried_fields,
    };

    fn unsigned(width: u32) -> AbiType {
        AbiType::Integer { sign: Sign::Unsigned, width }
    }

    fn signed(width: u32) -> AbiType {
        AbiType::Integer { sign: Sign::Signed, width }
    }

    fn two_to_the(bits: u32) -> BigUint {
        BigUint::from(1u8) << bits
    }

    #[test]
    fn every_element_below_the_selected_modulus_is_a_field_and_no_other() {
        for field in carried_fields() {
            let largest = field.modulus() - 1u8;
            let decoded = decode_scalar(largest.clone(), &AbiType::Field, field).unwrap();
            assert_eq!(encode_scalar(&decoded, &AbiType::Field, field), Ok(largest));

            for value in [field.modulus().clone(), field.modulus() + 1u8] {
                assert_eq!(
                    decode_scalar(value.clone(), &AbiType::Field, field),
                    Err(ScalarError::ExceedsFieldModulus { value, field: field.id() }),
                    "{}",
                    field.name()
                );
            }
        }
    }

    /// A container built in a wider linked field can hold an element of a narrower selected field
    /// that is not canonical there; encoding refuses it rather than reduce it.
    #[test]
    fn a_container_at_or_above_the_selected_modulus_is_refused() {
        for field in carried_fields() {
            let modulus = field.modulus();
            if modulus == FieldConfig::linked().modulus() {
                continue;
            }
            let value = InputValue::Field(container(modulus));
            assert_eq!(
                encode_scalar(&value, &AbiType::Field, field),
                Err(ScalarError::ExceedsFieldModulus { value: modulus.clone(), field: field.id() })
            );
        }
    }

    /// An integer's element is its bit pattern at the declared width, whatever the signedness; a
    /// boolean's is `0` or `1`.
    #[test]
    fn integers_and_booleans_take_exactly_the_patterns_of_their_type() {
        for field in carried_fields() {
            let widest = field.num_bits() - 1;
            for width in [2, 8, 63, widest] {
                for typ in [unsigned(width), signed(width)] {
                    let largest = two_to_the(width) - 1u8;
                    let decoded = decode_scalar(largest.clone(), &typ, field).unwrap();
                    assert_eq!(encode_scalar(&decoded, &typ, field), Ok(largest));
                    assert!(matches!(
                        decode_scalar(two_to_the(width), &typ, field),
                        Err(ScalarError::OutOfRange { .. })
                    ));
                }
            }
            for value in [0u8, 1] {
                let decoded = decode_scalar(BigUint::from(value), &AbiType::Boolean, field);
                assert_eq!(decoded, Ok(InputValue::Field(value.into())));
            }
            assert!(matches!(
                decode_scalar(BigUint::from(2u8), &AbiType::Boolean, field),
                Err(ScalarError::OutOfRange { .. })
            ));
        }
    }

    /// A type whose patterns reach the modulus keeps only the patterns below it.
    #[test]
    fn an_integer_wider_than_the_field_keeps_the_patterns_below_the_modulus() {
        for field in carried_fields() {
            let typ = unsigned(field.num_bits());
            let largest = field.modulus() - 1u8;
            assert!(decode_scalar(largest, &typ, field).is_ok());
            assert!(matches!(
                decode_scalar(field.modulus().clone(), &typ, field),
                Err(ScalarError::ExceedsFieldModulus { .. })
            ));
        }
    }

    #[test]
    fn aggregates_are_not_scalars() {
        let field = FieldConfig::linked();
        for typ in [
            AbiType::String { length: 1 },
            AbiType::Array { length: 1, typ: Box::new(AbiType::Field) },
            AbiType::Tuple { fields: vec![AbiType::Field] },
            AbiType::Struct { path: "S".into(), fields: vec![("a".into(), AbiType::Field)] },
        ] {
            assert_eq!(
                decode_scalar(BigUint::ZERO, &typ, field),
                Err(ScalarError::NotAScalar(typ.clone()))
            );
            let value = InputValue::Field(0u8.into());
            assert_eq!(encode_scalar(&value, &typ, field), Err(ScalarError::NotAScalar(typ)));
        }
        let value = InputValue::String("a".into());
        assert!(encode_scalar(&value, &AbiType::Field, field).is_err());
    }

    #[test]
    fn a_field_the_linked_element_cannot_hold_is_refused() {
        let linked = FieldConfig::linked().id();
        for field in uncarried_fields() {
            let refusal = ScalarError::FieldNotCarried { field: field.id(), linked };
            assert_eq!(decode_scalar(BigUint::ZERO, &AbiType::Field, field), Err(refusal.clone()));
            let value = InputValue::Field(0u8.into());
            assert_eq!(encode_scalar(&value, &AbiType::Field, field), Err(refusal));
        }
    }

    proptest! {
        #[test]
        fn decoding_then_encoding_returns_the_element(
            bytes in prop::collection::vec(any::<u8>(), 1..=32),
            width in 2u32..=253,
            is_signed: bool,
        ) {
            for field in carried_fields() {
                let width = width.min(field.num_bits() - 1);
                let pattern = BigUint::from_bytes_be(&bytes) % two_to_the(width);
                let typ = if is_signed { signed(width) } else { unsigned(width) };
                let decoded = decode_scalar(pattern.clone(), &typ, field).unwrap();
                prop_assert_eq!(encode_scalar(&decoded, &typ, field), Ok(pattern));
            }
        }
    }
}
