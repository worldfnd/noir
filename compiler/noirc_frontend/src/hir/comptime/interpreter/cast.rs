use crate::{
    Type,
    hir::comptime::{
        Integer, InterpreterError, Value, errors::IResult, integer::twos_complement_pattern,
        interpreter::ensure_integer_width_is_legal,
    },
};
use acvm::{FieldConfig, FieldValue};
use noirc_errors::Location;
use num_bigint::BigInt;

/// The width at which a value of `typ` is read as a two's complement pattern under `field`.
fn bit_size(field: FieldConfig, typ: &Type) -> u32 {
    match typ {
        Type::FieldElement => field.num_bits(),
        // The type of an evaluated value names its width outright.
        Type::Integer(_, width) => {
            width.constant_width().expect("ICE: a value's integer type has a concrete width")
        }
        Type::Bool => 1,
        _ => field.num_bits(),
    }
}

/// An integer target takes the source's two's complement pattern at the target width and reads it by the target's signedness; a `Field` target takes the source's own-width pattern modulo the field.
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
            let reduced = pattern.magnitude() % field.modulus();
            let reduced = FieldValue::try_from_biguint(reduced, field.id());
            Ok(Value::field(reduced.expect("a value reduced modulo the field is an element")))
        }
        Type::Integer(sign, width) => {
            let bits = width.evaluate_to_u32(location).map_err(|err| {
                InterpreterError::InvalidNumericGeneric { err: Box::new(err), location }
            })?;
            ensure_integer_width_is_legal(sign, bits, location)?;
            Ok(Value::Integer(Integer::wrapping_int(sign.is_signed(), bits, value)))
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
        let signed = Type::sint;
        let unsigned = Type::uint;

        let tests = [
            // Widen
            (Value::u8(255), unsigned(64), Value::u64(255)),
            (Value::u8(255), signed(64), Value::i64(255)),
            (Value::u64(u64::MAX), unsigned(128), Value::u128(u128::from(u64::MAX))),
            // Reinterpret as negative
            (Value::u8(255), signed(8), Value::i8(-1)),
            (field(255u32), signed(8), Value::i8(-1)),
            // Truncate
            (Value::u16(300), unsigned(8), Value::u8(44)),
            (Value::u16(300), signed(8), Value::i8(44)),
            (Value::u16(255), signed(8), Value::i8(-1)),
            (field(300u32), unsigned(8), Value::u8(44)),
            (field(300u32), signed(8), Value::i8(44)),
            (field(10u32), unsigned(16), Value::u16(10)),
            (field(256u32), unsigned(8), Value::u8(0)),
            (field(255u32), unsigned(8), Value::u8(255)),
            (Value::u128(u128::MAX), unsigned(64), Value::u64(u64::MAX)),
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
        let signed = Type::sint;
        let unsigned = Type::uint;

        let tests = [
            // Widen
            (Value::i8(127), unsigned(64), Value::u64(127)),
            (Value::i8(127), signed(64), Value::i64(127)),
            // Widen signed->unsigned: sign extend
            (Value::i8(-1), unsigned(16), Value::u16(65535)),
            (Value::i8(-100), unsigned(16), Value::u16(65436)),
            // A `Field` target takes the source's own-width pattern, so a negative value becomes positive and is never sign-extended to the field width.
            (Value::i8(-1), Type::FieldElement, field(255u32)),
            // Widen negative: sign extend
            (Value::i8(-1), signed(16), Value::i16(-1)),
            (Value::i8(-100), signed(16), Value::i16(-100)),
            // Reinterpret as positive
            (Value::i8(-100), unsigned(8), Value::u8(156)),
            // Truncate
            (Value::i16(300), unsigned(8), Value::u8(44)),
            (Value::i16(300), signed(8), Value::i8(44)),
            (Value::i16(255), signed(8), Value::i8(-1)),
            (Value::i16(i16::MIN + 5), signed(8), Value::i8(5)),
            (Value::i16(i16::MIN + 5), unsigned(8), Value::u8(5)),
            (Value::field(negated(1)), unsigned(8), Value::u8(0)),
            (Value::field(negated(1)), signed(8), Value::i8(0)),
            (Value::field(negated(2)), unsigned(16), Value::u16(65535)),
            (Value::field(negated(2)), signed(16), Value::i16(-1)),
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
        let unsigned = Type::uint;
        let signed = Type::sint;

        let tests = [
            (Value::u64(0xFFFF_FFFF_0000_0001), unsigned(128), Value::u128(0xFFFF_FFFF_0000_0001)),
            (Value::u64(u64::MAX), unsigned(64), Value::u64(u64::MAX)),
            (Value::u64(u64::MAX), signed(64), Value::i64(-1)),
            (Value::u64(u64::MAX), unsigned(8), Value::u8(0xFF)),
            (Value::i64(-1), unsigned(128), Value::u128(u128::MAX)),
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

    /// A `Field` target takes the source's own-width pattern modulo the field. `u64::MAX` is itself under bn254 and `2^32 - 2` under Goldilocks.
    #[test]
    fn casts_into_field_reduce_modulo_the_field() {
        let location = Location::dummy();
        for field_id in FieldId::ALL {
            let field = FieldConfig::new(field_id);
            let actual =
                evaluate_cast_one_step(field, &Type::FieldElement, location, Value::u64(u64::MAX));
            let expected = BigUint::from(u64::MAX) % field.modulus();
            let expected = FieldValue::try_from_biguint(expected, field_id).unwrap();
            assert_eq!(actual, Ok(Value::field(expected)), "{field_id}");
        }

        let goldilocks = FieldConfig::new(FieldId::Goldilocks);
        let actual =
            evaluate_cast_one_step(goldilocks, &Type::FieldElement, location, Value::u64(u64::MAX));
        let expected =
            FieldValue::try_from_biguint(BigUint::from(0xFFFF_FFFEu32), FieldId::Goldilocks);
        assert_eq!(actual, Ok(Value::field(expected.unwrap())));
    }

    /// The widths a cast across the field boundary is checked at: narrow ones, the host-limb
    /// boundaries, either side of the field's own width and of 256 bits, and widths far past
    /// any field.
    fn corner_widths(field: FieldConfig) -> Vec<u32> {
        let k = field.num_bits();
        let mut widths =
            vec![8, 32, 63, 64, 65, 127, 128, 129, k - 1, k, k + 1, 255, 256, 257, 1000, 16384];
        widths.sort_unstable();
        widths.dedup();
        widths
    }

    /// Patterns of a `bits`-wide integer around the modulus and its multiples, the field's width
    /// and the host limbs, each taken modulo `2^bits`.
    fn corner_patterns(bits: u32, field: FieldConfig) -> Vec<BigUint> {
        let one = BigUint::from(1u8);
        let pow2 = |exponent: u32| &one << exponent;
        let p = field.modulus();
        let k = field.num_bits();
        let top = pow2(bits) - 1u8;
        let multiple = (&top / p) * p;

        let mut patterns = vec![
            BigUint::ZERO,
            one.clone(),
            p - 2u8,
            p - 1u8,
            p.clone(),
            p + 1u8,
            p * 2u8 - 1u8,
            p * 2u8,
            p * 2u8 + 1u8,
            p * 3u8 - 1u8,
            p * 3u8,
            p * 3u8 + 1u8,
            pow2(k - 1),
            pow2(k) - 1u8,
            pow2(k),
            pow2(k) + 1u8,
            pow2(k + 1) - 1u8,
            pow2(64) - 1u8,
            pow2(64),
            pow2(128) - 1u8,
            pow2(256) - 1u8,
            pow2(256),
            &multiple + 1u8,
            multiple,
            top,
        ];
        for word in [0x55u8, 0xAA] {
            patterns.push(BigUint::from_bytes_le(&vec![word; bits.div_ceil(8) as usize]));
        }
        let mut patterns: Vec<_> = patterns.into_iter().map(|value| value % pow2(bits)).collect();
        patterns.sort_unstable();
        patterns.dedup();
        patterns
    }

    /// Canonical field elements around the top of the field, its halves and the host limbs.
    fn corner_elements(field: FieldConfig) -> Vec<BigUint> {
        let one = BigUint::from(1u8);
        let p = field.modulus();
        let k = field.num_bits();
        let mut elements = vec![
            BigUint::ZERO,
            one.clone(),
            p - 1u8,
            p - 2u8,
            (p - 1u8) >> 1u32,
            (p + 1u8) >> 1u32,
        ];
        for exponent in [7, 8, 31, 32, 33, 63, 64, 65, 127, 128, 129, k - 2, k - 1] {
            elements.push((&one << exponent) - 1u8);
            elements.push(&one << exponent);
        }
        elements.retain(|element| element < p);
        elements.sort_unstable();
        elements.dedup();
        elements
    }

    /// An unsigned integer of any width into `Field` is its value modulo the field, under every
    /// field and at widths on both sides of the field's own.
    #[test]
    fn casts_into_field_reduce_at_the_corners_of_every_width() {
        let location = Location::dummy();
        for field_id in FieldId::ALL {
            let field = FieldConfig::new(field_id);
            for bits in corner_widths(field) {
                for pattern in corner_patterns(bits, field) {
                    let source = BigInt::from(pattern.clone());
                    let source = Integer::try_from_bigint(&source, &Type::uint(bits), field_id);
                    let source = Value::Integer(source.expect("the pattern fits its width"));
                    let actual =
                        evaluate_cast_one_step(field, &Type::FieldElement, location, source);
                    let expected = &pattern % field.modulus();
                    let expected = FieldValue::try_from_biguint(expected, field_id).unwrap();
                    assert_eq!(
                        actual,
                        Ok(Value::field(expected)),
                        "{field_id}: {pattern} as a u{bits}"
                    );
                }
            }
        }
    }

    /// A `Field` into an integer of any width keeps the low bits of its canonical value, read by
    /// the target's signedness, and is the value itself once the width holds the whole field.
    #[test]
    fn casts_out_of_field_keep_the_low_bits_at_the_corners_of_every_width() {
        let location = Location::dummy();
        for field_id in FieldId::ALL {
            let field = FieldConfig::new(field_id);
            for bits in corner_widths(field) {
                for element in corner_elements(field) {
                    let low = &element % (BigUint::from(1u8) << bits);
                    let source = FieldValue::try_from_biguint(element.clone(), field_id).unwrap();

                    let actual = evaluate_cast_one_step(
                        field,
                        &Type::uint(bits),
                        location,
                        Value::field(source.clone()),
                    );
                    let expected =
                        Integer::Int { signed: false, bits, value: BigInt::from(low.clone()) };
                    assert_eq!(
                        actual,
                        Ok(Value::Integer(expected)),
                        "{field_id}: {element} as u{bits}"
                    );

                    let actual = evaluate_cast_one_step(
                        field,
                        &Type::sint(bits),
                        location,
                        Value::field(source),
                    );
                    let mut value = BigInt::from(low);
                    if value >= (BigInt::from(1) << (bits - 1)) {
                        value -= BigInt::from(1) << bits;
                    }
                    let expected = Integer::Int { signed: true, bits, value };
                    assert_eq!(
                        actual,
                        Ok(Value::Integer(expected)),
                        "{field_id}: {element} as i{bits}"
                    );
                }
            }
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
        prop::sample::select(vec![
            Type::uint(8),
            Type::uint(16),
            Type::uint(32),
            Type::uint(64),
            Type::uint(128),
            Type::sint(8),
            Type::sint(16),
            Type::sint(32),
            Type::sint(64),
            Type::FieldElement,
        ])
    }

    proptest! {
        /// An integer target reads the source's two's complement pattern at the target width by the target's signedness; a `Field` target takes the source's own-width pattern modulo `p`.
        #[test]
        fn casts_follow_the_bit_pattern_model_at_every_width(
            (source, value, source_bits) in source_value_and_width(),
            target in target_types(),
        ) {
            let expected = match &target {
                Type::Integer(sign, bits) => {
                    let width = bits.constant_width().unwrap();
                    let modulus = BigInt::from(1) << width;
                    let low = ((&value % &modulus) + &modulus) % &modulus;
                    let value = if *sign == Signedness::Signed && low >= (BigInt::from(1) << (width - 1)) {
                        low - (BigInt::from(1) << width)
                    } else {
                        low
                    };
                    Value::Integer(Integer::try_from_bigint(&value, &target, FieldId::linked()).unwrap())
                }
                Type::FieldElement => {
                    let modulus = BigInt::from(1) << source_bits;
                    let low = ((&value % &modulus) + &modulus) % &modulus;
                    field(low.magnitude() % FieldElement::modulus())
                }
                _ => unreachable!("integer and field targets only"),
            };
            let actual = evaluate_cast_one_step(FieldConfig::linked(), &target, Location::dummy(), source.clone());
            prop_assert_eq!(actual, Ok(expected), "{:?} as {}", source, target);
        }
    }
}
