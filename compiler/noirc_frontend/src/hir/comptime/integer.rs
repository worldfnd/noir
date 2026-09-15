//! Comptime numeric values carry the configured field. The free conversion functions use the linked backend's `FieldElement`.

use std::fmt::Display;
use std::hash::{Hash, Hasher};

use acvm::{AcirField, FieldElement, FieldId, FieldValue};
use num_bigint::{BigInt, BigUint, Sign};
use num_traits::{One, Signed, Zero};

use crate::{
    Kind, Type,
    ast::{ExpressionKind, IntegerBitSize, Literal},
    hir_def::expr::{HirExpression, HirLiteral},
    shared::Signedness,
    token::{IntegerTypeSuffix, Token},
};

/// Converts a `FieldElement` to the `BigInt` holding its canonical (non-negative) representative.
pub(crate) fn field_to_bigint(value: &FieldElement) -> BigInt {
    BigInt::from_biguint(Sign::Plus, BigUint::from_bytes_be(&value.to_be_bytes()))
}

/// Converts a `BigInt` to a `FieldElement`, encoding negative values via field negation:
/// `-7` becomes `-FieldElement::from(7)`.
///
/// Returns `None` if the magnitude is at or above the linked field's modulus.
fn try_bigint_to_field(value: &BigInt) -> Option<FieldElement> {
    if *value.magnitude() >= FieldElement::modulus() {
        return None;
    }
    let field = FieldElement::from_be_bytes_reduce(&value.magnitude().to_bytes_be());
    Some(if value.sign() == Sign::Minus { -field } else { field })
}

/// Converts a `BigInt` to a `FieldElement`, like `try_bigint_to_field`, for values which
/// are known to be canonical (with a magnitude less than the field modulus).
///
/// Panics if the magnitude is at or above the linked field's modulus. Callers must check this bound before lowering exact literals.
pub fn bigint_to_field(value: &BigInt) -> FieldElement {
    try_bigint_to_field(value)
        .unwrap_or_else(|| panic!("ICE: value does not fit in the field: {value}"))
}

/// Converts a `FieldElement` to a `BigInt`, choosing the sign which gives the shorter
/// decimal representation, mirroring `FieldElement`'s `Display` impl. This keeps values
/// which encode negative numbers via field negation displaying as negative numbers.
pub fn field_to_signed_bigint(value: &FieldElement) -> BigInt {
    let positive = field_to_bigint(value);
    let negated = field_to_bigint(&-*value);
    if negated.to_string().len() < positive.to_string().len() { -negated } else { positive }
}

/// A comptime field element or integer.
///
/// `Int` values must be canonical: `[0, 2^bits)` unsigned or `[-2^(bits-1), 2^(bits-1))` signed.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Integer {
    Field(FieldValue),
    Int { signed: bool, bits: u32, value: BigInt },
}

fn fits(signed: bool, bits: u32, value: &BigInt) -> bool {
    let bits = u64::from(bits);
    match (signed, value.sign()) {
        (_, Sign::NoSign) => true,
        (false, Sign::Minus) => false,
        (false, Sign::Plus) => value.bits() <= bits,
        (true, Sign::Plus) => value.bits() < bits,
        // `-v` fits when `|v| - 1` fits below the sign bit.
        (true, Sign::Minus) => (value.magnitude() - 1u8).bits() < bits,
    }
}

macro_rules! int_constructors {
    ($($name:ident: $ty:ty => $signed:literal;)*) => {
        $(
            pub fn $name(value: $ty) -> Integer {
                Integer::Int { signed: $signed, bits: <$ty>::BITS, value: BigInt::from(value) }
            }
        )*
    };
}

macro_rules! exact_width_accessors {
    ($($name:ident: $ty:ty;)*) => {
        $(
            /// Returns the value only if its unsigned type matches exactly.
            pub fn $name(&self) -> Option<$ty> {
                match self {
                    Integer::Int { signed: false, bits: <$ty>::BITS, value } => {
                        Some(<$ty>::try_from(value).expect("a canonical value fits its own width"))
                    }
                    _ => None,
                }
            }
        )*
    };
}

impl Integer {
    int_constructors! {
        u8: u8 => false;
        u16: u16 => false;
        u32: u32 => false;
        u64: u64 => false;
        u128: u128 => false;
        i8: i8 => true;
        i16: i16 => true;
        i32: i32 => true;
        i64: i64 => true;
    }

    exact_width_accessors! {
        as_u8: u8;
        as_u32: u32;
        as_u64: u64;
    }

    /// Returns `None` if `value` is outside the range for this signedness and width.
    pub fn int(signed: bool, bits: u32, value: BigInt) -> Option<Integer> {
        fits(signed, bits, &value).then_some(Integer::Int { signed, bits, value })
    }

    /// Keeps the low `bits` bits, interpreted as two's complement when signed.
    pub(crate) fn wrapping_int(signed: bool, bits: u32, value: BigInt) -> Integer {
        let modulus = BigInt::one() << bits;
        let mut value = ((value % &modulus) + &modulus) % &modulus;
        if signed && value >= (BigInt::one() << (bits - 1)) {
            value -= modulus;
        }
        Integer::Int { signed, bits, value }
    }

    /// Returns whether an integer is negative. Field elements always return `false`.
    pub fn is_negative(&self) -> bool {
        match self {
            Integer::Field(_) => false,
            Integer::Int { value, .. } => value.is_negative(),
        }
    }

    /// The signedness and width of an integer; `None` for a field element.
    pub fn signed_and_bits(&self) -> Option<(bool, u32)> {
        match self {
            Integer::Field(_) => None,
            Integer::Int { signed, bits, .. } => Some((*signed, *bits)),
        }
    }

    pub fn get_type(&self) -> Type {
        match self {
            Integer::Field(_) => Type::FieldElement,
            Integer::Int { signed, bits, .. } => {
                let sign = if *signed { Signedness::Signed } else { Signedness::Unsigned };
                let bits = IntegerBitSize::try_from(*bits).unwrap_or_else(|_| {
                    panic!("ICE: the type checker names no {bits}-bit integer")
                });
                Type::Integer(sign, bits)
            }
        }
    }

    /// Returns this value's numeric kind.
    pub fn numeric_kind(&self) -> Kind {
        Kind::Numeric(Box::new(self.get_type()))
    }

    /// Converts this [Integer] to a [BigInt]. Negative signed values become negative
    /// bigints, and field values which display as negative numbers (see
    /// [field_to_signed_bigint]) also become negative bigints.
    pub fn to_bigint(&self) -> BigInt {
        match self {
            Integer::Field(value) => value.to_signed_bigint(),
            Integer::Int { value, .. } => value.clone(),
        }
    }

    pub(crate) fn into_expression_kind(self) -> ExpressionKind {
        let suffix = self.integer_type_suffix();
        ExpressionKind::Literal(Literal::Integer(self.to_bigint(), suffix))
    }

    pub(crate) fn into_hir_expression(self) -> HirExpression {
        HirExpression::Literal(HirLiteral::Integer(self.to_bigint()))
    }

    pub(crate) fn into_tokens(self) -> Vec<Token> {
        let suffix = self.integer_type_suffix();
        vec![Token::Int(self.to_bigint(), suffix)]
    }

    pub fn is_zero(&self) -> bool {
        match self {
            Integer::Field(value) => value.is_zero(),
            Integer::Int { value, .. } => value.is_zero(),
        }
    }

    pub fn is_one(&self) -> bool {
        match self {
            Integer::Field(value) => value.is_one(),
            Integer::Int { value, .. } => value.is_one(),
        }
    }

    /// Converts a field value to the given numeric type, returning `None` if it does not fit.
    /// Signed targets choose the shorter of the positive and negated spellings by bit length, with ties positive.
    pub fn try_from_field(value: FieldValue, typ: &Type) -> Option<Integer> {
        match typ.follow_bindings_shallow().as_ref() {
            Type::FieldElement => Some(Integer::Field(value)),
            Type::Integer(Signedness::Unsigned, size) => {
                Integer::int(false, u32::from(*size), value.to_bigint())
            }
            Type::Integer(Signedness::Signed, size) => {
                let positive = value.to_bigint();
                let negated = (-value).to_bigint();
                let reading = if negated.bits() < positive.bits() { -negated } else { positive };
                Integer::int(true, u32::from(*size), reading)
            }
            _ => None,
        }
    }

    /// Try to create an integer of the given type from the given bigint value.
    ///
    /// Returns `None` if the given type is not a field or integer, or
    /// if the value does not fit the type. Field values may be negative,
    /// in which case they are encoded via field negation.
    pub(crate) fn try_from_bigint(value: &BigInt, typ: &Type, field: FieldId) -> Option<Integer> {
        match typ.follow_bindings_shallow().as_ref() {
            Type::FieldElement => FieldValue::try_from_bigint(value, field).map(Integer::Field),
            Type::Integer(sign, size) => {
                Integer::int(sign.is_signed(), u32::from(*size), value.clone())
            }
            _ => None,
        }
    }

    /// Create an [Integer] from the given [IntegerTypeSuffix]. Returns `None` if the
    /// given value does not fit in the desired integer type.
    pub(crate) fn try_from_bigint_and_type_suffix(
        value: &BigInt,
        suffix: IntegerTypeSuffix,
        field: FieldId,
    ) -> Option<Integer> {
        Self::try_from_bigint(value, &suffix.as_type(), field)
    }

    /// The literal suffix naming this value's type, if the language has one.
    pub fn integer_type_suffix(&self) -> Option<IntegerTypeSuffix> {
        match self {
            Integer::Field(_) => Some(IntegerTypeSuffix::Field),
            Integer::Int { signed, bits, .. } => match (signed, bits) {
                (true, 8) => Some(IntegerTypeSuffix::I8),
                (true, 16) => Some(IntegerTypeSuffix::I16),
                (true, 32) => Some(IntegerTypeSuffix::I32),
                (true, 64) => Some(IntegerTypeSuffix::I64),
                (false, 8) => Some(IntegerTypeSuffix::U8),
                (false, 16) => Some(IntegerTypeSuffix::U16),
                (false, 32) => Some(IntegerTypeSuffix::U32),
                (false, 64) => Some(IntegerTypeSuffix::U64),
                (false, 128) => Some(IntegerTypeSuffix::U128),
                _ => None,
            },
        }
    }

    /// `self < rhs`, or `None` for fields or mismatched integer types.
    pub fn lt(&self, rhs: &Self) -> Option<bool> {
        let (lhs, rhs) = Self::same_int_type(self, rhs)?;
        Some(lhs < rhs)
    }

    /// `self <= rhs`, or `None` for fields or mismatched integer types.
    pub fn lte(&self, rhs: &Self) -> Option<bool> {
        let (lhs, rhs) = Self::same_int_type(self, rhs)?;
        Some(lhs <= rhs)
    }

    /// Left shift, discarding bits outside the width. Returns `None` for fields or `amount >= bits`.
    pub fn checked_shl(&self, amount: u32) -> Option<Integer> {
        let Integer::Int { signed, bits, value } = self else { return None };
        (amount < *bits).then(|| Integer::wrapping_int(*signed, *bits, value << amount))
    }

    /// Right shift, sign-extending signed values. Returns `None` for fields or `amount >= bits`.
    pub fn checked_shr(&self, amount: u32) -> Option<Integer> {
        let Integer::Int { signed, bits, value } = self else { return None };
        if amount >= *bits {
            return None;
        }
        Integer::int(*signed, *bits, value >> amount)
    }

    /// Bitwise complement within the integer's width. Returns `None` for fields.
    pub fn not(&self) -> Option<Integer> {
        let Integer::Int { signed, bits, value } = self else { return None };
        let complement = if *signed { -value - 1 } else { (BigInt::one() << *bits) - 1 - value };
        Integer::int(*signed, *bits, complement)
    }

    fn same_int_type<'a>(lhs: &'a Self, rhs: &'a Self) -> Option<(&'a BigInt, &'a BigInt)> {
        match (lhs, rhs) {
            (
                Integer::Int { signed, bits, value: lhs },
                Integer::Int { signed: rhs_signed, bits: rhs_bits, value: rhs },
            ) if signed == rhs_signed && bits == rhs_bits => Some((lhs, rhs)),
            _ => None,
        }
    }

    /// Applies `op` to matching integer types and checks the result's range.
    fn checked(self, rhs: Self, op: impl FnOnce(BigInt, BigInt) -> Option<BigInt>) -> Option<Self> {
        match (self, rhs) {
            (
                Integer::Int { signed, bits, value: lhs },
                Integer::Int { signed: rhs_signed, bits: rhs_bits, value: rhs },
            ) if signed == rhs_signed && bits == rhs_bits => {
                Integer::int(signed, bits, op(lhs, rhs)?)
            }
            _ => None,
        }
    }
}

impl Display for Integer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Integer::Field(value) => write!(f, "{}", value.to_short_hex()),
            Integer::Int { value, .. } => write!(f, "{value}"),
        }
    }
}

impl std::fmt::Debug for Integer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Integer::Field(value) => write!(f, "{}_Field", value.to_short_hex()),
            Integer::Int { signed, bits, value } => {
                let sign = if *signed { 'i' } else { 'u' };
                write!(f, "{value}_{sign}{bits}")
            }
        }
    }
}

impl Hash for Integer {
    // Keep type tags and primitive encodings stable for comptime reflection.
    fn hash<H: Hasher>(&self, state: &mut H) {
        macro_rules! at_width {
            ($index:expr, $ty:ty, $value:expr) => {{
                $index.hash(state);
                <$ty>::try_from($value).expect("a canonical value fits its own width").hash(state);
            }};
        }
        match self {
            Integer::Field(value) => {
                0isize.hash(state);
                value.hash(state);
            }
            Integer::Int { signed, bits, value } => match (signed, bits) {
                (true, 8) => at_width!(1isize, i8, value),
                (true, 16) => at_width!(2isize, i16, value),
                (true, 32) => at_width!(3isize, i32, value),
                (true, 64) => at_width!(4isize, i64, value),
                (false, 8) => at_width!(5isize, u8, value),
                (false, 16) => at_width!(6isize, u16, value),
                (false, 32) => at_width!(7isize, u32, value),
                (false, 64) => at_width!(8isize, u64, value),
                (false, 128) => at_width!(9isize, u128, value),
                _ => {
                    10isize.hash(state);
                    signed.hash(state);
                    bits.hash(state);
                    value.hash(state);
                }
            },
        }
    }
}

impl std::ops::Add for Integer {
    type Output = Option<Self>;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Integer::Field(lhs), Integer::Field(rhs)) => Some(Integer::Field(lhs + rhs)),
            (lhs, rhs) => lhs.checked(rhs, |lhs, rhs| Some(lhs + rhs)),
        }
    }
}

impl std::ops::Sub for Integer {
    type Output = Option<Self>;

    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Integer::Field(lhs), Integer::Field(rhs)) => Some(Integer::Field(lhs - rhs)),
            (lhs, rhs) => lhs.checked(rhs, |lhs, rhs| Some(lhs - rhs)),
        }
    }
}

impl std::ops::Mul for Integer {
    type Output = Option<Self>;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Integer::Field(lhs), Integer::Field(rhs)) => Some(Integer::Field(lhs * rhs)),
            (lhs, rhs) => lhs.checked(rhs, |lhs, rhs| Some(lhs * rhs)),
        }
    }
}

impl std::ops::Div for Integer {
    type Output = Option<Self>;

    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Integer::Field(lhs), Integer::Field(rhs)) => lhs.checked_div(&rhs).map(Integer::Field),
            (lhs, rhs) => lhs.checked(rhs, |lhs, rhs| (!rhs.is_zero()).then(|| lhs / rhs)),
        }
    }
}

impl std::ops::Rem for Integer {
    type Output = Option<Self>;

    fn rem(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (
                Integer::Int { signed, bits, value: lhs },
                Integer::Int { signed: rhs_signed, bits: rhs_bits, value: rhs },
            ) if signed == rhs_signed && bits == rhs_bits => {
                if rhs.is_zero() {
                    return None;
                }
                // Reject `MIN % -1` to match checked integer remainder.
                if !fits(signed, bits, &(&lhs / &rhs)) {
                    return None;
                }
                Integer::int(signed, bits, lhs % rhs)
            }
            _ => None,
        }
    }
}

impl std::ops::BitAnd for Integer {
    type Output = Option<Self>;

    fn bitand(self, rhs: Self) -> Self::Output {
        self.checked(rhs, |lhs, rhs| Some(lhs & rhs))
    }
}

impl std::ops::BitOr for Integer {
    type Output = Option<Self>;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.checked(rhs, |lhs, rhs| Some(lhs | rhs))
    }
}

impl std::ops::BitXor for Integer {
    type Output = Option<Self>;

    fn bitxor(self, rhs: Self) -> Self::Output {
        self.checked(rhs, |lhs, rhs| Some(lhs ^ rhs))
    }
}

impl std::ops::Neg for Integer {
    type Output = Option<Self>;

    fn neg(self) -> Self::Output {
        match self {
            Integer::Field(rhs) => Some(Integer::Field(-rhs)),
            Integer::Int { signed: false, .. } => None,
            Integer::Int { signed: true, bits, value } => Integer::int(true, bits, -value),
        }
    }
}

#[cfg(test)]
mod tests {
    use acvm::{AcirField, FieldElement, FieldId, FieldValue};
    use proptest::prelude::*;

    use num_bigint::{BigInt, BigUint, Sign};

    use super::{
        Integer, bigint_to_field, field_to_bigint, field_to_signed_bigint, try_bigint_to_field,
    };
    use crate::Type;
    use crate::ast::IntegerBitSize;
    use crate::shared::Signedness;

    /// A value of the field this build is linked against, the only field the tests below can
    /// compare with a `FieldElement`.
    fn linked(value: impl Into<BigUint>) -> FieldValue {
        FieldValue::from_linked_element(FieldElement::from_be_bytes_reduce(
            &value.into().to_bytes_be(),
        ))
    }

    /// The field value a native integer encodes, negatives by field negation.
    fn encoded(integer: &Integer) -> FieldValue {
        FieldValue::try_from_bigint(&integer.to_bigint(), FieldId::linked())
            .expect("a native integer is canonical in every supported field")
    }

    proptest! {
        // Field subtraction is the inverse of addition: (a - b) + b == a
        // Explicit edge cases: (0,1) tests modular wrapping to p-1, (0,0) tests zero-zero
        #[test]
        fn field_subtraction_is_inverse_of_addition(
            (a, b) in prop_oneof![
                Just((0u64, 1u64)),
                Just((0u64, 0u64)),
                Just((1u64, 1u64)),
                (any::<u64>(), any::<u64>()),
            ]
        ) {
            let fa = Integer::Field(linked(a));
            let fb = Integer::Field(linked(b));
            let result = (fa.clone() - fb.clone()).unwrap();
            let check = (result + fb).unwrap();
            assert_eq!(check, fa);
        }

        // Field negation is the additive inverse: (-a) + a == 0
        // Explicit edge case: negation of zero should be zero
        #[test]
        fn field_negation_is_additive_inverse(
            a in prop_oneof![Just(0u64), any::<u64>()]
        ) {
            let fa = Integer::Field(linked(a));
            let neg_a = (-fa.clone()).unwrap();
            let check = (neg_a + fa).unwrap();
            assert_eq!(check, Integer::Field(FieldValue::zero(FieldId::linked())));
        }

        // Field values are never considered negative
        #[test]
        fn field_is_never_negative(a: u64) {
            assert!(!Integer::Field(linked(a)).is_negative());
        }

        // Round-trip: Integer -> field value -> try_from_field -> same Integer
        // Tests that negative signed values survive the field encoding round-trip.
        #[test]
        fn i8_try_from_field_roundtrips(a: i8) {
            let integer = Integer::i8(a);
            let typ = Type::Integer(Signedness::Signed, IntegerBitSize::Eight);
            assert_eq!(Integer::try_from_field(encoded(&integer), &typ), Some(integer));
        }

        #[test]
        fn i16_try_from_field_roundtrips(a: i16) {
            let integer = Integer::i16(a);
            let typ = Type::Integer(Signedness::Signed, IntegerBitSize::Sixteen);
            assert_eq!(Integer::try_from_field(encoded(&integer), &typ), Some(integer));
        }

        #[test]
        fn i32_try_from_field_roundtrips(a: i32) {
            let integer = Integer::i32(a);
            let typ = Type::Integer(Signedness::Signed, IntegerBitSize::ThirtyTwo);
            assert_eq!(Integer::try_from_field(encoded(&integer), &typ), Some(integer));
        }

        #[test]
        fn i64_try_from_field_roundtrips(a: i64) {
            let integer = Integer::i64(a);
            let typ = Type::Integer(Signedness::Signed, IntegerBitSize::SixtyFour);
            assert_eq!(Integer::try_from_field(encoded(&integer), &typ), Some(integer));
        }

        #[test]
        fn u8_try_from_field_roundtrips(a: u8) {
            let integer = Integer::u8(a);
            let typ = Type::Integer(Signedness::Unsigned, IntegerBitSize::Eight);
            assert_eq!(Integer::try_from_field(encoded(&integer), &typ), Some(integer));
        }

        #[test]
        fn field_try_from_field_roundtrips(a: u64) {
            let value = linked(a);
            let integer = Integer::Field(value.clone());
            assert_eq!(Integer::try_from_field(value, &Type::FieldElement), Some(integer));
        }
    }

    #[test]
    fn type_mismatch_returns_none() {
        for (a, b) in [(Integer::i8(1), Integer::i16(1)), (Integer::u8(1), Integer::i8(1))] {
            assert_eq!(a.clone() + b.clone(), None);
            assert_eq!(a.clone() - b.clone(), None);
            assert_eq!(a.clone() * b.clone(), None);
            assert_eq!(a.clone() / b.clone(), None);
            assert_eq!(a.clone() % b.clone(), None);
            assert_eq!(a.clone() & b.clone(), None);
            assert_eq!(a.clone() | b.clone(), None);
            assert_eq!(a.clone() ^ b.clone(), None);
            assert_eq!(a.lt(&b), None);
            assert_eq!(a.lte(&b), None);
        }
    }

    #[test]
    fn field_division_by_zero() {
        let a = Integer::Field(linked(5u64));
        let b = Integer::Field(FieldValue::zero(FieldId::linked()));
        // Division by zero must honor the `None`-on-failure contract rather than
        // silently canonicalizing to the field-element inverse of zero (which is zero).
        assert_eq!(a / b, None);
    }

    #[test]
    fn field_remainder_rejected() {
        let a = Integer::Field(linked(10u64));
        let b = Integer::Field(linked(3u64));
        assert_eq!(a % b, None);
    }

    #[test]
    fn field_lt_is_unordered() {
        let neg_one = Integer::Field(encoded(&Integer::i64(-1)));
        let zero = Integer::Field(FieldValue::zero(FieldId::linked()));
        assert_eq!(neg_one.lt(&zero), None);
        assert_eq!(zero.lt(&neg_one), None);
    }

    #[test]
    fn field_lte_is_unordered() {
        let neg_one = Integer::Field(encoded(&Integer::i64(-1)));
        let zero = Integer::Field(FieldValue::zero(FieldId::linked()));
        assert_eq!(neg_one.lte(&zero), None);
        assert_eq!(zero.lte(&neg_one), None);
    }

    proptest! {
        // Round-trip: FieldElement -> BigInt -> FieldElement
        #[test]
        fn field_to_bigint_roundtrips(a: u64) {
            let field = FieldElement::from(u128::from(a));
            assert_eq!(bigint_to_field(&field_to_bigint(&field)), field);
        }

        // Negative bigints are encoded via field negation: -x == -FieldElement::from(x)
        #[test]
        fn bigint_to_field_encodes_negatives_via_field_negation(a: u64) {
            prop_assume!(BigUint::from(a) < FieldElement::modulus());
            let value = -BigInt::from(a);
            assert_eq!(bigint_to_field(&value), -FieldElement::from(u128::from(a)));
        }

        // `a` is below `10^18`, so `-a` is the shorter spelling under every supported modulus.
        #[test]
        fn field_to_signed_bigint_recovers_negatives(a in 1..10u64.pow(18)) {
            let field = -FieldElement::from(u128::from(a));
            assert_eq!(field_to_signed_bigint(&field), -BigInt::from(a));
        }

        #[test]
        fn try_from_bigint_matches_rust_conversion_for_i8(a: i128) {
            let value = BigInt::from(a);
            let typ = Type::Integer(Signedness::Signed, IntegerBitSize::Eight);
            let expected = i8::try_from(a).ok().map(Integer::i8);
            assert_eq!(Integer::try_from_bigint(&value, &typ, FieldId::linked()), expected);
        }

        #[test]
        fn try_from_bigint_matches_rust_conversion_for_u8(a: i128) {
            let value = BigInt::from(a);
            let typ = Type::Integer(Signedness::Unsigned, IntegerBitSize::Eight);
            let expected = u8::try_from(a).ok().map(Integer::u8);
            assert_eq!(Integer::try_from_bigint(&value, &typ, FieldId::linked()), expected);
        }

        // Integer -> BigInt -> Integer round-trips through try_from_bigint
        #[test]
        fn to_bigint_roundtrips_for_i64(a: i64) {
            let integer = Integer::i64(a);
            let typ = Type::Integer(Signedness::Signed, IntegerBitSize::SixtyFour);
            let value = Integer::try_from_bigint(&integer.to_bigint(), &typ, FieldId::linked());
            assert_eq!(value, Some(integer));
        }
    }

    #[test]
    fn try_from_bigint_respects_boundaries() {
        use IntegerBitSize::*;
        use Signedness::*;

        let i8_type = Type::Integer(Signed, Eight);
        let field = FieldId::linked();
        let from = |value: BigInt, typ: &Type| Integer::try_from_bigint(&value, typ, field);

        assert_eq!(from(BigInt::from(-128), &i8_type), Some(Integer::i8(-128)));
        assert_eq!(from(BigInt::from(127), &i8_type), Some(Integer::i8(127)));
        assert_eq!(from(BigInt::from(-129), &i8_type), None);
        assert_eq!(from(BigInt::from(128), &i8_type), None);

        let u8_type = Type::Integer(Unsigned, Eight);
        assert_eq!(from(BigInt::from(0), &u8_type), Some(Integer::u8(0)));
        assert_eq!(from(BigInt::from(255), &u8_type), Some(Integer::u8(255)));
        assert_eq!(from(BigInt::from(256), &u8_type), None);
        assert_eq!(from(BigInt::from(-1), &u8_type), None);

        let i64_type = Type::Integer(Signed, SixtyFour);
        assert_eq!(from(BigInt::from(i64::MIN), &i64_type), Some(Integer::i64(i64::MIN)));
        assert_eq!(from(BigInt::from(i64::MIN) - 1, &i64_type), None);

        let u128_type = Type::Integer(Unsigned, HundredTwentyEight);
        assert_eq!(from(BigInt::from(u128::MAX), &u128_type), Some(Integer::u128(u128::MAX)));
        assert_eq!(from(BigInt::from(u128::MAX) + 1, &u128_type), None);
        assert_eq!(from(BigInt::from(-1), &u128_type), None);
    }

    #[test]
    fn bigint_to_field_does_not_reduce_non_canonical_values() {
        let modulus = BigInt::from_biguint(Sign::Plus, FieldElement::modulus());

        // The largest canonical magnitudes convert, for both signs
        let max_canonical = modulus.clone() - 1;
        assert_eq!(try_bigint_to_field(&max_canonical), Some(-FieldElement::one()));
        assert_eq!(try_bigint_to_field(&-max_canonical), Some(FieldElement::one()));

        // Magnitudes of at least the modulus are rejected rather than reduced
        assert_eq!(try_bigint_to_field(&modulus), None);
        assert_eq!(try_bigint_to_field(&-(modulus.clone())), None);
        assert_eq!(try_bigint_to_field(&(modulus.clone() + 1)), None);
        assert_eq!(
            Integer::try_from_bigint(&modulus, &Type::FieldElement, FieldId::linked()),
            None
        );
    }

    /// Reading an integer out of a `Field` value must agree with the linked element's own
    /// conversions, which the back half and the ABI still use.
    #[test]
    fn try_from_field_agrees_with_the_linked_element() {
        use IntegerBitSize::*;
        use Signedness::*;

        let mut values = vec![BigUint::ZERO, BigUint::from(1u8), FieldElement::modulus() - 1u8];
        for power in [7u32, 8, 15, 16, 31, 32, 63, 64, 127, 128] {
            let value = BigUint::from(1u8) << power;
            if value < FieldElement::modulus() {
                values.push(FieldElement::modulus() - &value);
                values.push(value);
            }
        }

        for value in values {
            let element = FieldElement::from_be_bytes_reduce(&value.to_bytes_be());
            let field = linked(value);
            let cases = [
                (Type::Integer(Unsigned, Eight), u8::try_from(element).ok().map(Integer::u8)),
                (Type::Integer(Unsigned, Sixteen), u16::try_from(element).ok().map(Integer::u16)),
                (Type::Integer(Unsigned, ThirtyTwo), u32::try_from(element).ok().map(Integer::u32)),
                (Type::Integer(Unsigned, SixtyFour), u64::try_from(element).ok().map(Integer::u64)),
                (
                    Type::Integer(Unsigned, HundredTwentyEight),
                    u128::try_from(element).ok().map(Integer::u128),
                ),
                (Type::Integer(Signed, Eight), i8::try_from(element).ok().map(Integer::i8)),
                (Type::Integer(Signed, Sixteen), i16::try_from(element).ok().map(Integer::i16)),
                (Type::Integer(Signed, ThirtyTwo), i32::try_from(element).ok().map(Integer::i32)),
                (Type::Integer(Signed, SixtyFour), i64::try_from(element).ok().map(Integer::i64)),
            ];
            for (typ, expected) in cases {
                assert_eq!(
                    Integer::try_from_field(field.clone(), &typ),
                    expected,
                    "{element} as {typ}"
                );
            }
        }
    }

    /// A field narrower than the target type reads `2^63` as a negative number, since `p - 2^63`
    /// is the shorter spelling. The linked element cannot show this under bn254.
    #[test]
    fn a_narrow_field_reads_the_shorter_spelling_as_negative() {
        let field = FieldId::Goldilocks;
        let two_to_the_63 = BigUint::from(1u8) << 63;
        let value = FieldValue::try_from_biguint(two_to_the_63, field).unwrap();
        let i64_type = Type::Integer(Signedness::Signed, IntegerBitSize::SixtyFour);

        // p - 2^63 == 2^63 - 2^32 + 1
        assert_eq!(
            Integer::try_from_field(value.clone(), &i64_type),
            Some(Integer::i64(-9223372032559808513))
        );
        let u64_type = Type::Integer(Signedness::Unsigned, IntegerBitSize::SixtyFour);
        assert_eq!(
            Integer::try_from_field(value, &u64_type),
            Some(Integer::u64(1 << 63)),
            "an unsigned target reads the value itself"
        );
    }

    #[test]
    #[should_panic(expected = "ICE: value does not fit in the field")]
    fn bigint_to_field_panics_on_non_canonical_values() {
        let modulus = BigInt::from_biguint(Sign::Plus, FieldElement::modulus());
        bigint_to_field(&modulus);
    }

    #[test]
    fn wide_integer_arithmetic_and_bounds() {
        let power = |exponent: u32| {
            Integer::int(false, 256, BigInt::from(1) << exponent).expect("below 2^256")
        };

        assert_eq!(power(200) * power(55), Some(power(255)));
        assert_eq!(power(255) + power(255), None, "2^256 does not fit a u256");
        assert_eq!(power(255).checked_shr(255), Some(power(0)));
        assert_eq!(
            power(1).checked_shl(255),
            Some(Integer::int(false, 256, BigInt::ZERO).unwrap())
        );
        assert_eq!(power(0).checked_shl(256), None, "a shift by the width is refused");
        assert_eq!(Integer::int(false, 256, BigInt::from(1) << 256u32), None);

        let minimum = -(BigInt::from(1) << 255u32);
        assert!(Integer::int(true, 256, minimum.clone()).is_some());
        assert_eq!(Integer::int(true, 256, minimum - 1), None);

        assert_eq!(power(255).to_string(), (BigInt::from(1) << 255u32).to_string());
        assert_eq!(format!("{:?}", power(3)), "8_u256");
        assert_eq!(power(3).integer_type_suffix(), None);
    }

    #[test]
    fn hash_preserves_reflection_values() {
        use std::hash::{DefaultHasher, Hash, Hasher};

        fn hash_of(value: impl Hash) -> u64 {
            let mut hasher = DefaultHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        }

        let field = linked(5u64);
        assert_eq!(hash_of(Integer::Field(field.clone())), hash_of((0isize, field)));
        assert_eq!(hash_of(Integer::i8(-3)), hash_of((1isize, -3i8)));
        assert_eq!(hash_of(Integer::i16(-3)), hash_of((2isize, -3i16)));
        assert_eq!(hash_of(Integer::i32(-3)), hash_of((3isize, -3i32)));
        assert_eq!(hash_of(Integer::i64(-3)), hash_of((4isize, -3i64)));
        assert_eq!(hash_of(Integer::u8(3)), hash_of((5isize, 3u8)));
        assert_eq!(hash_of(Integer::u16(3)), hash_of((6isize, 3u16)));
        assert_eq!(hash_of(Integer::u32(3)), hash_of((7isize, 3u32)));
        assert_eq!(hash_of(Integer::u64(3)), hash_of((8isize, 3u64)));
        assert_eq!(hash_of(Integer::u128(3)), hash_of((9isize, 3u128)));
        assert_ne!(hash_of(Integer::u8(3)), hash_of(Integer::u16(3)));
    }

    macro_rules! agrees_with_rust {
        ($($test:ident: $ty:ident, $signed:literal;)*) => {
            $(
                proptest! {
                    #[test]
                    fn $test(a: $ty, b: $ty, amount in 0u32..200) {
                        let (x, y) = (Integer::$ty(a), Integer::$ty(b));
                        prop_assert_eq!(x.clone() + y.clone(), a.checked_add(b).map(Integer::$ty));
                        prop_assert_eq!(x.clone() - y.clone(), a.checked_sub(b).map(Integer::$ty));
                        prop_assert_eq!(x.clone() * y.clone(), a.checked_mul(b).map(Integer::$ty));
                        prop_assert_eq!(x.clone() / y.clone(), a.checked_div(b).map(Integer::$ty));
                        prop_assert_eq!(x.clone() % y.clone(), a.checked_rem(b).map(Integer::$ty));
                        let negated = if $signed { a.checked_neg().map(Integer::$ty) } else { None };
                        prop_assert_eq!(-x.clone(), negated);
                        prop_assert_eq!(x.lt(&y), Some(a < b));
                        prop_assert_eq!(x.lte(&y), Some(a <= b));
                        prop_assert_eq!(x.clone() & y.clone(), Some(Integer::$ty(a & b)));
                        prop_assert_eq!(x.clone() | y.clone(), Some(Integer::$ty(a | b)));
                        prop_assert_eq!(x.clone() ^ y.clone(), Some(Integer::$ty(a ^ b)));
                        prop_assert_eq!(x.not(), Some(Integer::$ty(!a)));
                        prop_assert_eq!(x.checked_shl(amount), a.checked_shl(amount).map(Integer::$ty));
                        prop_assert_eq!(x.checked_shr(amount), a.checked_shr(amount).map(Integer::$ty));
                        prop_assert_eq!(x.to_bigint(), BigInt::from(a));
                        prop_assert_eq!(x.is_negative(), BigInt::from(a).sign() == Sign::Minus);
                        prop_assert_eq!(x.to_string(), a.to_string());
                        prop_assert_eq!(format!("{:?}", x), format!("{}_{}", a, stringify!($ty)));
                        prop_assert_eq!(x.signed_and_bits(), Some(($signed, <$ty>::BITS)));
                    }
                }
            )*
        };
    }

    agrees_with_rust! {
        i8_agrees_with_rust: i8, true;
        i16_agrees_with_rust: i16, true;
        i32_agrees_with_rust: i32, true;
        i64_agrees_with_rust: i64, true;
        u8_agrees_with_rust: u8, false;
        u16_agrees_with_rust: u16, false;
        u32_agrees_with_rust: u32, false;
        u64_agrees_with_rust: u64, false;
        u128_agrees_with_rust: u128, false;
    }

    #[test]
    fn fields_have_no_bitwise_or_shift_operations() {
        let one = Integer::Field(linked(1u64));
        assert_eq!(one.clone() & one.clone(), None);
        assert_eq!(one.clone() | one.clone(), None);
        assert_eq!(one.clone() ^ one.clone(), None);
        assert_eq!(one.not(), None);
        assert_eq!(one.checked_shl(1), None);
        assert_eq!(one.checked_shr(1), None);
        assert_eq!(one.signed_and_bits(), None);
    }

    #[test]
    fn exact_width_accessors_read_only_their_own_type() {
        assert_eq!(Integer::u32(7).as_u32(), Some(7));
        assert_eq!(Integer::u8(7).as_u32(), None);
        assert_eq!(Integer::i32(7).as_u32(), None);
        assert_eq!(Integer::Field(linked(7u64)).as_u32(), None);
        assert_eq!(Integer::u8(7).as_u8(), Some(7));
        assert_eq!(Integer::u64(7).as_u64(), Some(7));
        assert_eq!(Integer::u64(7).as_u8(), None);
    }
}
