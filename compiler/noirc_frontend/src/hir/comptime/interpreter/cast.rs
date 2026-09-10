use crate::{
    Type,
    hir::comptime::{Integer, InterpreterError, Value, errors::IResult},
};
use acvm::{FieldConfig, FieldValue};
use noirc_errors::Location;
use num_bigint::BigInt;

/// The width at which a value of `typ` is read as a two's complement pattern under `field`.
fn bit_size(field: FieldConfig, typ: &Type) -> u32 {
    match typ {
        Type::FieldElement => field.num_bits(),
        Type::Integer(_, bit_size) => u32::from(bit_size.bit_size()),
        Type::Bool => 1,
        _ => field.num_bits(),
    }
}

/// The two's complement pattern of `value` at `bits` bits.
fn twos_complement_pattern(value: &BigInt, bits: u32) -> BigInt {
    let modulus = BigInt::from(1) << bits;
    ((value % &modulus) + &modulus) % &modulus
}

/// An integer target takes the source's two's complement pattern at the target width and reads it by the target's signedness; a `Field` target takes the source's own-width pattern exactly, and is an error if that pattern is not below the modulus (the type checker admits only source types whose every pattern is).
pub(crate) fn evaluate_cast_one_step(
    field: FieldConfig,
    output_type: &Type,
    location: Location,
    evaluated_lhs: Value,
) -> IResult<Value> {
    let lhs_type = evaluated_lhs.get_type().into_owned();
    let value = match evaluated_lhs {
        Value::Integer(Integer::Field(value)) => value.to_bigint(),
        Value::Integer(integer) => integer.to_bigint(),
        Value::Bool(value) => BigInt::from(u8::from(value)),
        _ => return Err(InterpreterError::NonNumericCasted { typ: lhs_type, location }),
    };
    match output_type.follow_bindings() {
        Type::FieldElement => {
            let pattern = twos_complement_pattern(&value, bit_size(field, &lhs_type));
            FieldValue::try_from_bigint(&pattern, field.id()).map(Value::field).ok_or_else(|| {
                InterpreterError::IntegerOutOfRangeForType {
                    value: pattern,
                    typ: Type::FieldElement,
                    location,
                }
            })
        }
        typ @ Type::Integer(sign, size) => {
            let bits = u32::from(size.bit_size());
            let mut value = twos_complement_pattern(&value, bits);
            if sign.is_signed() && value >= (BigInt::from(1) << (bits - 1)) {
                value -= BigInt::from(1) << bits;
            }
            Integer::try_from_bigint(&value, &typ, field.id())
                .map(Value::Integer)
                .ok_or(InterpreterError::TypeUnsupported { typ, location })
        }
        Type::Bool if lhs_type == Type::Bool => Ok(Value::Bool(value != BigInt::ZERO)),
        // Numeric conversions to booleans must use `!= 0`
        Type::Bool => Err(InterpreterError::CannotCastNumericToBool { typ: lhs_type, location }),
        typ => Err(InterpreterError::CastToNonNumericType { typ, location }),
    }
}

#[cfg(test)]
mod tests {
    use acvm::{AcirField, FieldElement, FieldId, FieldValue};

    /// A `Field` value of the field this build is linked against, which is the field the tests
    /// below compile for.
    fn field(value: impl Into<BigUint>) -> Value {
        Value::field(linked(value))
    }

    fn linked(value: impl Into<BigUint>) -> FieldValue {
        FieldValue::try_from_biguint(value.into(), FieldId::linked())
            .expect("the test values are below every modulus")
    }

    /// The negation of `value` in the linked field.
    fn negated(value: u32) -> FieldValue {
        -linked(value)
    }
    use noirc_errors::Location;
    use num_bigint::BigUint;
    use proptest::prelude::*;

    use super::*;
    use crate::ast::IntegerBitSize;
    use crate::shared::Signedness;

    #[test]
    fn smoke_test() {
        let location = Location::dummy();
        let typ = Type::FieldElement;

        let lhs_values = [
            field(1u32),
            Value::Bool(true),
            Value::u8(1),
            Value::u16(1),
            Value::u32(1),
            Value::u64(1),
            Value::u128(1),
            Value::i8(1),
            Value::i16(1),
            Value::i32(1),
            Value::i64(1),
        ];

        for lhs in lhs_values {
            assert_eq!(
                evaluate_cast_one_step(FieldConfig::linked(), &typ, location, lhs),
                Ok(field(1u32))
            );
        }
    }

    #[test]
    fn unsigned_casts() {
        let location = Location::dummy();
        let signed = |size| Type::Integer(Signedness::Signed, size);
        let unsigned = |size| Type::Integer(Signedness::Unsigned, size);

        use IntegerBitSize::*;
        let tests = [
            // Widen
            (Value::u8(255), unsigned(SixtyFour), Value::u64(255)),
            (Value::u8(255), signed(SixtyFour), Value::i64(255)),
            (Value::u64(u64::MAX), unsigned(HundredTwentyEight), Value::u128(u128::from(u64::MAX))),
            // Reinterpret as negative
            (Value::u8(255), signed(Eight), Value::i8(-1)),
            (field(255u32), signed(Eight), Value::i8(-1)),
            // Truncate
            (Value::u16(300), unsigned(Eight), Value::u8(44)),
            (Value::u16(300), signed(Eight), Value::i8(44)),
            (Value::u16(255), signed(Eight), Value::i8(-1)),
            (field(300u32), unsigned(Eight), Value::u8(44)),
            (field(300u32), signed(Eight), Value::i8(44)),
            (field(10u32), unsigned(Sixteen), Value::u16(10)),
            (field(256u32), unsigned(Eight), Value::u8(0)),
            (field(255u32), unsigned(Eight), Value::u8(255)),
            (Value::u128(u128::MAX), unsigned(SixtyFour), Value::u64(u64::MAX)),
            // Casting Field -> Field should be a no-op
            (field(4u32), Type::FieldElement, field(4u32)),
            (Value::field(negated(4)), Type::FieldElement, Value::field(negated(4))),
        ];

        for (lhs, typ, expected) in tests {
            let actual = evaluate_cast_one_step(FieldConfig::linked(), &typ, location, lhs.clone());
            assert_eq!(
                actual,
                Ok(expected.clone()),
                "{lhs:?} as {typ}, expected {expected:?}, got {actual:?}"
            );
        }
    }

    #[test]
    fn signed_casts() {
        let location = Location::dummy();
        let signed = |size| Type::Integer(Signedness::Signed, size);
        let unsigned = |size| Type::Integer(Signedness::Unsigned, size);

        use IntegerBitSize::*;
        let tests = [
            // Widen
            (Value::i8(127), unsigned(SixtyFour), Value::u64(127)),
            (Value::i8(127), signed(SixtyFour), Value::i64(127)),
            // Widen signed->unsigned: sign extend
            (Value::i8(-1), unsigned(Sixteen), Value::u16(65535)),
            (Value::i8(-100), unsigned(Sixteen), Value::u16(65436)),
            // A `Field` target takes the source's own-width pattern, so a negative value becomes positive and is never sign-extended to the field width.
            (Value::i8(-1), Type::FieldElement, field(255u32)),
            // Widen negative: sign extend
            (Value::i8(-1), signed(Sixteen), Value::i16(-1)),
            (Value::i8(-100), signed(Sixteen), Value::i16(-100)),
            // Reinterpret as positive
            (Value::i8(-100), unsigned(Eight), Value::u8(156)),
            // Truncate
            (Value::i16(300), unsigned(Eight), Value::u8(44)),
            (Value::i16(300), signed(Eight), Value::i8(44)),
            (Value::i16(255), signed(Eight), Value::i8(-1)),
            (Value::i16(i16::MIN + 5), signed(Eight), Value::i8(5)),
            (Value::i16(i16::MIN + 5), unsigned(Eight), Value::u8(5)),
            (Value::field(negated(1)), unsigned(Eight), Value::u8(0)),
            (Value::field(negated(1)), signed(Eight), Value::i8(0)),
            (Value::field(negated(2)), unsigned(Sixteen), Value::u16(65535)),
            (Value::field(negated(2)), signed(Sixteen), Value::i16(-1)),
        ];

        for (lhs, typ, expected) in tests {
            let actual = evaluate_cast_one_step(FieldConfig::linked(), &typ, location, lhs.clone());
            assert_eq!(
                actual,
                Ok(expected.clone()),
                "{lhs:?} as {typ}, expected {expected:?}, got {actual:?}"
            );
        }
    }

    #[test]
    fn bool_cast() {
        let location = Location::dummy();
        let lhs = field(0u32);
        let actual = evaluate_cast_one_step(FieldConfig::linked(), &Type::Bool, location, lhs);
        assert!(matches!(actual, Err(InterpreterError::CannotCastNumericToBool { .. })));
    }

    /// Goldilocks' `p` is below `u64::MAX`; casting through a `FieldElement` would reduce every one of these.
    #[test]
    fn integer_casts_do_not_reduce_modulo_the_field() {
        let location = Location::dummy();
        let unsigned = |size| Type::Integer(Signedness::Unsigned, size);
        let signed = |size| Type::Integer(Signedness::Signed, size);

        use IntegerBitSize::*;
        let tests = [
            (
                Value::u64(0xFFFF_FFFF_0000_0001),
                unsigned(HundredTwentyEight),
                Value::u128(0xFFFF_FFFF_0000_0001),
            ),
            (Value::u64(u64::MAX), unsigned(SixtyFour), Value::u64(u64::MAX)),
            (Value::u64(u64::MAX), signed(SixtyFour), Value::i64(-1)),
            (Value::u64(u64::MAX), unsigned(Eight), Value::u8(0xFF)),
            (Value::i64(-1), unsigned(HundredTwentyEight), Value::u128(u128::MAX)),
        ];

        for (lhs, typ, expected) in tests {
            let actual = evaluate_cast_one_step(FieldConfig::linked(), &typ, location, lhs.clone());
            assert_eq!(
                actual,
                Ok(expected.clone()),
                "{lhs:?} as {typ}, expected {expected:?}, got {actual:?}"
            );
        }
    }

    /// A `Field` target never reduces: a pattern below the modulus converts exactly and a pattern at or above it is an error. The largest `u128` below `p` converts under every field; `u64::MAX` fits under bn254 and exceeds Goldilocks' `p`.
    #[test]
    fn field_casts_are_exact_or_refused() {
        let location = Location::dummy();
        let modulus = FieldElement::modulus();
        let largest = BigUint::from(u128::MAX).min(&modulus - 1u8);
        let expected_value = largest.clone();

        let source = Value::u128(u128::try_from(largest).unwrap());
        let actual =
            evaluate_cast_one_step(FieldConfig::linked(), &Type::FieldElement, location, source);
        assert_eq!(actual, Ok(Value::field(linked(expected_value))));

        let actual = evaluate_cast_one_step(
            FieldConfig::linked(),
            &Type::FieldElement,
            location,
            Value::u64(u64::MAX),
        );
        if BigUint::from(u64::MAX) < modulus {
            assert_eq!(actual, Ok(field(u64::MAX)));
        } else {
            assert!(
                matches!(
                    actual,
                    Err(InterpreterError::IntegerOutOfRangeForType {
                        ref value,
                        typ: Type::FieldElement,
                        ..
                    }) if *value == BigInt::from(u64::MAX)
                ),
                "{actual:?}"
            );
        }
    }

    /// A source value with its mathematical value and its width in bits.
    fn source_value_and_width() -> impl Strategy<Value = (Value, BigInt, u32)> {
        prop_oneof![
            any::<u8>().prop_map(|x| (Value::u8(x), BigInt::from(x), 8)),
            any::<u16>().prop_map(|x| (Value::u16(x), BigInt::from(x), 16)),
            any::<u32>().prop_map(|x| (Value::u32(x), BigInt::from(x), 32)),
            any::<u64>().prop_map(|x| (Value::u64(x), BigInt::from(x), 64)),
            any::<u128>().prop_map(|x| (Value::u128(x), BigInt::from(x), 128)),
            any::<i8>().prop_map(|x| (Value::i8(x), BigInt::from(x), 8)),
            any::<i16>().prop_map(|x| (Value::i16(x), BigInt::from(x), 16)),
            any::<i32>().prop_map(|x| (Value::i32(x), BigInt::from(x), 32)),
            any::<i64>().prop_map(|x| (Value::i64(x), BigInt::from(x), 64)),
            any::<u64>()
                .prop_filter("below the modulus", |x| BigUint::from(*x) < FieldElement::modulus())
                .prop_map(|x| (field(x), BigInt::from(x), FieldElement::max_num_bits())),
            any::<bool>().prop_map(|x| (Value::Bool(x), BigInt::from(u8::from(x)), 1)),
        ]
    }

    fn target_types() -> impl Strategy<Value = Type> {
        use IntegerBitSize::*;
        use Signedness::*;
        prop::sample::select(vec![
            Type::Integer(Unsigned, Eight),
            Type::Integer(Unsigned, Sixteen),
            Type::Integer(Unsigned, ThirtyTwo),
            Type::Integer(Unsigned, SixtyFour),
            Type::Integer(Unsigned, HundredTwentyEight),
            Type::Integer(Signed, Eight),
            Type::Integer(Signed, Sixteen),
            Type::Integer(Signed, ThirtyTwo),
            Type::Integer(Signed, SixtyFour),
            Type::FieldElement,
        ])
    }

    proptest! {
        /// An integer target reads the source's two's complement pattern at the target width by the target's signedness; a `Field` target takes the source's own-width pattern exactly when it is below `p` and is an error otherwise.
        #[test]
        fn casts_follow_the_bit_pattern_model_at_every_width(
            (source, value, source_bits) in source_value_and_width(),
            target in target_types(),
        ) {
            let expected = match &target {
                Type::Integer(sign, bits) => {
                    let width = u32::from(bits.bit_size());
                    let modulus = BigInt::from(1) << width;
                    let low = ((&value % &modulus) + &modulus) % &modulus;
                    let value = if *sign == Signedness::Signed && low >= (BigInt::from(1) << (width - 1)) {
                        low - (BigInt::from(1) << width)
                    } else {
                        low
                    };
                    Some(Value::Integer(Integer::try_from_bigint(&value, &target, FieldId::linked()).unwrap()))
                }
                Type::FieldElement => {
                    let modulus = BigInt::from(1) << source_bits;
                    let low = ((&value % &modulus) + &modulus) % &modulus;
                    (low < BigInt::from(FieldElement::modulus())).then(|| {
                        field(low.magnitude().clone())
                    })
                }
                _ => unreachable!("integer and field targets only"),
            };
            let actual = evaluate_cast_one_step(FieldConfig::linked(), &target, Location::dummy(), source.clone());
            match expected {
                Some(expected) => prop_assert_eq!(actual, Ok(expected), "{:?} as {}", source, target),
                None => prop_assert!(
                    matches!(actual, Err(InterpreterError::IntegerOutOfRangeForType { .. })),
                    "{:?} as {} gave {:?}", source, target, actual
                ),
            }
        }
    }
}
