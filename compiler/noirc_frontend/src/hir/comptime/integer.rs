//! Comptime numeric values carry the configured field. The free conversion functions use the linked backend's `FieldElement`.

use std::fmt::Display;

use acvm::{AcirField, FieldElement, FieldId, FieldValue};
use num_bigint::{BigInt, BigUint, Sign};

use crate::{
    Kind, Type,
    ast::{ExpressionKind, IntegerBitSize},
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

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Integer {
    Field(FieldValue),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
}

impl Integer {
    /// Returns whether this integer is strictly less than zero.
    ///
    /// Only the signed variants can be negative. Unsigned integers cannot represent a negative
    /// value, and a Noir `Field` has no signedness: `Integer::Field` wraps an element of a prime
    /// field, which has no notion of sign. Both therefore return `false`.
    pub fn is_negative(&self) -> bool {
        match self {
            Integer::I8(x) => *x < 0,
            Integer::I16(x) => *x < 0,
            Integer::I32(x) => *x < 0,
            Integer::I64(x) => *x < 0,
            Integer::Field(_)
            | Integer::U8(_)
            | Integer::U16(_)
            | Integer::U32(_)
            | Integer::U64(_)
            | Integer::U128(_) => false,
        }
    }

    pub fn get_type(&self) -> Type {
        match self {
            Integer::Field(_) => Type::FieldElement,
            Integer::I8(_) => Type::Integer(Signedness::Signed, IntegerBitSize::Eight),
            Integer::I16(_) => Type::Integer(Signedness::Signed, IntegerBitSize::Sixteen),
            Integer::I32(_) => Type::Integer(Signedness::Signed, IntegerBitSize::ThirtyTwo),
            Integer::I64(_) => Type::Integer(Signedness::Signed, IntegerBitSize::SixtyFour),
            Integer::U8(_) => Type::Integer(Signedness::Unsigned, IntegerBitSize::Eight),
            Integer::U16(_) => Type::Integer(Signedness::Unsigned, IntegerBitSize::Sixteen),
            Integer::U32(_) => Type::Integer(Signedness::Unsigned, IntegerBitSize::ThirtyTwo),
            Integer::U64(_) => Type::Integer(Signedness::Unsigned, IntegerBitSize::SixtyFour),
            Integer::U128(_) => {
                Type::Integer(Signedness::Unsigned, IntegerBitSize::HundredTwentyEight)
            }
        }
    }

    /// Returns the type of this kind wrapped in `Kind::Numeric`
    pub fn numeric_kind(&self) -> Kind {
        Kind::Numeric(Box::new(self.get_type()))
    }

    /// Converts this [Integer] to a [BigInt]. Negative signed values become negative
    /// bigints, and field values which display as negative numbers (see
    /// [field_to_signed_bigint]) also become negative bigints.
    pub fn to_bigint(&self) -> BigInt {
        match self {
            Integer::Field(value) => value.to_signed_bigint(),
            Integer::I8(value) => (*value).into(),
            Integer::I16(value) => (*value).into(),
            Integer::I32(value) => (*value).into(),
            Integer::I64(value) => (*value).into(),
            Integer::U8(value) => (*value).into(),
            Integer::U16(value) => (*value).into(),
            Integer::U32(value) => (*value).into(),
            Integer::U64(value) => (*value).into(),
            Integer::U128(value) => (*value).into(),
        }
    }

    pub(crate) fn into_expression_kind(self) -> ExpressionKind {
        use crate::ast::Literal::Integer as Int;
        use ExpressionKind::Literal;
        match self {
            Integer::Field(value) => {
                Literal(Int(value.to_signed_bigint(), Some(IntegerTypeSuffix::Field)))
            }
            Integer::I8(value) => Literal(Int(value.into(), Some(IntegerTypeSuffix::I8))),
            Integer::I16(value) => Literal(Int(value.into(), Some(IntegerTypeSuffix::I16))),
            Integer::I32(value) => Literal(Int(value.into(), Some(IntegerTypeSuffix::I32))),
            Integer::I64(value) => Literal(Int(value.into(), Some(IntegerTypeSuffix::I64))),
            Integer::U8(value) => Literal(Int(value.into(), Some(IntegerTypeSuffix::U8))),
            Integer::U16(value) => Literal(Int(value.into(), Some(IntegerTypeSuffix::U16))),
            Integer::U32(value) => Literal(Int(value.into(), Some(IntegerTypeSuffix::U32))),
            Integer::U64(value) => Literal(Int(value.into(), Some(IntegerTypeSuffix::U64))),
            Integer::U128(value) => Literal(Int(value.into(), Some(IntegerTypeSuffix::U128))),
        }
    }

    pub(crate) fn into_hir_expression(self) -> HirExpression {
        match self {
            Integer::Field(value) => {
                HirExpression::Literal(HirLiteral::Integer(value.to_signed_bigint()))
            }
            Integer::I8(value) => HirExpression::Literal(HirLiteral::Integer(value.into())),
            Integer::I16(value) => HirExpression::Literal(HirLiteral::Integer(value.into())),
            Integer::I32(value) => HirExpression::Literal(HirLiteral::Integer(value.into())),
            Integer::I64(value) => HirExpression::Literal(HirLiteral::Integer(value.into())),
            Integer::U8(value) => HirExpression::Literal(HirLiteral::Integer(value.into())),
            Integer::U16(value) => HirExpression::Literal(HirLiteral::Integer(value.into())),
            Integer::U32(value) => HirExpression::Literal(HirLiteral::Integer(value.into())),
            Integer::U64(value) => HirExpression::Literal(HirLiteral::Integer(value.into())),
            Integer::U128(value) => HirExpression::Literal(HirLiteral::Integer(value.into())),
        }
    }

    pub(crate) fn into_tokens(self) -> Vec<Token> {
        match self {
            Integer::U8(value) => {
                vec![Token::Int(value.into(), Some(IntegerTypeSuffix::U8))]
            }
            Integer::U16(value) => {
                vec![Token::Int(value.into(), Some(IntegerTypeSuffix::U16))]
            }
            Integer::U32(value) => {
                vec![Token::Int(value.into(), Some(IntegerTypeSuffix::U32))]
            }
            Integer::U64(value) => {
                vec![Token::Int(value.into(), Some(IntegerTypeSuffix::U64))]
            }
            Integer::U128(value) => {
                vec![Token::Int(value.into(), Some(IntegerTypeSuffix::U128))]
            }
            Integer::I8(value) => {
                vec![Token::Int(value.into(), Some(IntegerTypeSuffix::I8))]
            }
            Integer::I16(value) => {
                vec![Token::Int(value.into(), Some(IntegerTypeSuffix::I16))]
            }
            Integer::I32(value) => {
                vec![Token::Int(value.into(), Some(IntegerTypeSuffix::I32))]
            }
            Integer::I64(value) => {
                vec![Token::Int(value.into(), Some(IntegerTypeSuffix::I64))]
            }
            Integer::Field(value) => {
                vec![Token::Int(value.to_signed_bigint(), Some(IntegerTypeSuffix::Field))]
            }
        }
    }

    pub fn is_zero(&self) -> bool {
        match self {
            Integer::Field(value) => value.is_zero(),
            Integer::I8(value) => *value == 0,
            Integer::I16(value) => *value == 0,
            Integer::I32(value) => *value == 0,
            Integer::I64(value) => *value == 0,
            Integer::U8(value) => *value == 0,
            Integer::U16(value) => *value == 0,
            Integer::U32(value) => *value == 0,
            Integer::U64(value) => *value == 0,
            Integer::U128(value) => *value == 0,
        }
    }

    pub fn is_one(&self) -> bool {
        match self {
            Integer::Field(value) => value.is_one(),
            Integer::I8(value) => *value == 1,
            Integer::I16(value) => *value == 1,
            Integer::I32(value) => *value == 1,
            Integer::I64(value) => *value == 1,
            Integer::U8(value) => *value == 1,
            Integer::U16(value) => *value == 1,
            Integer::U32(value) => *value == 1,
            Integer::U64(value) => *value == 1,
            Integer::U128(value) => *value == 1,
        }
    }

    /// Try to create an integer of the given type from the given field value, which encodes a
    /// negative number as the negation of its magnitude.
    ///
    /// An unsigned target takes the value when it is below `2^width`; a signed target reads the
    /// shorter of the value's two spellings, so `p - 1` becomes `-1`. Returns `None` if the given
    /// type is not a field or integer, or if the value does not fit the type.
    pub fn try_from_field(value: FieldValue, typ: &Type) -> Option<Integer> {
        use IntegerBitSize::*;
        use Signedness::*;
        match typ.follow_bindings_shallow().as_ref() {
            Type::FieldElement => Some(Integer::Field(value)),
            Type::Integer(Unsigned, Eight) => value.try_to_u32()?.try_into().ok().map(Integer::U8),
            Type::Integer(Unsigned, Sixteen) => {
                value.try_to_u32()?.try_into().ok().map(Integer::U16)
            }
            Type::Integer(Unsigned, ThirtyTwo) => value.try_to_u32().map(Integer::U32),
            Type::Integer(Unsigned, SixtyFour) => value.try_to_u64().map(Integer::U64),
            Type::Integer(Unsigned, HundredTwentyEight) => value.try_into_u128().map(Integer::U128),
            Type::Integer(Signed, Eight) => value.try_into_i128()?.try_into().ok().map(Integer::I8),
            Type::Integer(Signed, Sixteen) => {
                value.try_into_i128()?.try_into().ok().map(Integer::I16)
            }
            Type::Integer(Signed, ThirtyTwo) => {
                value.try_into_i128()?.try_into().ok().map(Integer::I32)
            }
            Type::Integer(Signed, SixtyFour) => {
                value.try_into_i128()?.try_into().ok().map(Integer::I64)
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
        use IntegerBitSize::*;
        use Signedness::*;
        match typ.follow_bindings_shallow().as_ref() {
            Type::FieldElement => FieldValue::try_from_bigint(value, field).map(Integer::Field),
            Type::Integer(Unsigned, Eight) => u8::try_from(value).ok().map(Integer::U8),
            Type::Integer(Unsigned, Sixteen) => u16::try_from(value).ok().map(Integer::U16),
            Type::Integer(Unsigned, ThirtyTwo) => u32::try_from(value).ok().map(Integer::U32),
            Type::Integer(Unsigned, SixtyFour) => u64::try_from(value).ok().map(Integer::U64),
            Type::Integer(Unsigned, HundredTwentyEight) => {
                u128::try_from(value).ok().map(Integer::U128)
            }
            Type::Integer(Signed, Eight) => i8::try_from(value).ok().map(Integer::I8),
            Type::Integer(Signed, Sixteen) => i16::try_from(value).ok().map(Integer::I16),
            Type::Integer(Signed, ThirtyTwo) => i32::try_from(value).ok().map(Integer::I32),
            Type::Integer(Signed, SixtyFour) => i64::try_from(value).ok().map(Integer::I64),
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

    pub fn integer_type_suffix(&self) -> IntegerTypeSuffix {
        match self {
            Integer::Field(_) => IntegerTypeSuffix::Field,
            Integer::I8(_) => IntegerTypeSuffix::I8,
            Integer::I16(_) => IntegerTypeSuffix::I16,
            Integer::I32(_) => IntegerTypeSuffix::I32,
            Integer::I64(_) => IntegerTypeSuffix::I64,
            Integer::U8(_) => IntegerTypeSuffix::U8,
            Integer::U16(_) => IntegerTypeSuffix::U16,
            Integer::U32(_) => IntegerTypeSuffix::U32,
            Integer::U64(_) => IntegerTypeSuffix::U64,
            Integer::U128(_) => IntegerTypeSuffix::U128,
        }
    }
}

impl Display for Integer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Integer::Field(value) => write!(f, "{}", value.to_short_hex()),
            Integer::I8(value) => write!(f, "{value}"),
            Integer::I16(value) => write!(f, "{value}"),
            Integer::I32(value) => write!(f, "{value}"),
            Integer::I64(value) => write!(f, "{value}"),
            Integer::U8(value) => write!(f, "{value}"),
            Integer::U16(value) => write!(f, "{value}"),
            Integer::U32(value) => write!(f, "{value}"),
            Integer::U64(value) => write!(f, "{value}"),
            Integer::U128(value) => write!(f, "{value}"),
        }
    }
}

impl std::fmt::Debug for Integer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Integer::Field(value) => write!(f, "{}_Field", value.to_short_hex()),
            Integer::I8(value) => write!(f, "{value}_i8"),
            Integer::I16(value) => write!(f, "{value}_i16"),
            Integer::I32(value) => write!(f, "{value}_i32"),
            Integer::I64(value) => write!(f, "{value}_i64"),
            Integer::U8(value) => write!(f, "{value}_u8"),
            Integer::U16(value) => write!(f, "{value}_u16"),
            Integer::U32(value) => write!(f, "{value}_u32"),
            Integer::U64(value) => write!(f, "{value}_u64"),
            Integer::U128(value) => write!(f, "{value}_u128"),
        }
    }
}

// All [Integer] operations return [None] on overflow or type mismatch
impl std::ops::Add for Integer {
    type Output = Option<Self>;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Integer::Field(lhs), Integer::Field(rhs)) => Some(Integer::Field(lhs + rhs)),
            (Integer::U8(lhs), Integer::U8(rhs)) => lhs.checked_add(rhs).map(Integer::U8),
            (Integer::U16(lhs), Integer::U16(rhs)) => lhs.checked_add(rhs).map(Integer::U16),
            (Integer::U32(lhs), Integer::U32(rhs)) => lhs.checked_add(rhs).map(Integer::U32),
            (Integer::U64(lhs), Integer::U64(rhs)) => lhs.checked_add(rhs).map(Integer::U64),
            (Integer::U128(lhs), Integer::U128(rhs)) => lhs.checked_add(rhs).map(Integer::U128),
            (Integer::I8(lhs), Integer::I8(rhs)) => lhs.checked_add(rhs).map(Integer::I8),
            (Integer::I16(lhs), Integer::I16(rhs)) => lhs.checked_add(rhs).map(Integer::I16),
            (Integer::I32(lhs), Integer::I32(rhs)) => lhs.checked_add(rhs).map(Integer::I32),
            (Integer::I64(lhs), Integer::I64(rhs)) => lhs.checked_add(rhs).map(Integer::I64),
            _ => None,
        }
    }
}

impl std::ops::Sub for Integer {
    type Output = Option<Self>;

    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Integer::Field(lhs), Integer::Field(rhs)) => Some(Integer::Field(lhs - rhs)),
            (Integer::U8(lhs), Integer::U8(rhs)) => lhs.checked_sub(rhs).map(Integer::U8),
            (Integer::U16(lhs), Integer::U16(rhs)) => lhs.checked_sub(rhs).map(Integer::U16),
            (Integer::U32(lhs), Integer::U32(rhs)) => lhs.checked_sub(rhs).map(Integer::U32),
            (Integer::U64(lhs), Integer::U64(rhs)) => lhs.checked_sub(rhs).map(Integer::U64),
            (Integer::U128(lhs), Integer::U128(rhs)) => lhs.checked_sub(rhs).map(Integer::U128),
            (Integer::I8(lhs), Integer::I8(rhs)) => lhs.checked_sub(rhs).map(Integer::I8),
            (Integer::I16(lhs), Integer::I16(rhs)) => lhs.checked_sub(rhs).map(Integer::I16),
            (Integer::I32(lhs), Integer::I32(rhs)) => lhs.checked_sub(rhs).map(Integer::I32),
            (Integer::I64(lhs), Integer::I64(rhs)) => lhs.checked_sub(rhs).map(Integer::I64),
            _ => None,
        }
    }
}

impl std::ops::Mul for Integer {
    type Output = Option<Self>;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Integer::Field(lhs), Integer::Field(rhs)) => Some(Integer::Field(lhs * rhs)),
            (Integer::U8(lhs), Integer::U8(rhs)) => lhs.checked_mul(rhs).map(Integer::U8),
            (Integer::U16(lhs), Integer::U16(rhs)) => lhs.checked_mul(rhs).map(Integer::U16),
            (Integer::U32(lhs), Integer::U32(rhs)) => lhs.checked_mul(rhs).map(Integer::U32),
            (Integer::U64(lhs), Integer::U64(rhs)) => lhs.checked_mul(rhs).map(Integer::U64),
            (Integer::U128(lhs), Integer::U128(rhs)) => lhs.checked_mul(rhs).map(Integer::U128),
            (Integer::I8(lhs), Integer::I8(rhs)) => lhs.checked_mul(rhs).map(Integer::I8),
            (Integer::I16(lhs), Integer::I16(rhs)) => lhs.checked_mul(rhs).map(Integer::I16),
            (Integer::I32(lhs), Integer::I32(rhs)) => lhs.checked_mul(rhs).map(Integer::I32),
            (Integer::I64(lhs), Integer::I64(rhs)) => lhs.checked_mul(rhs).map(Integer::I64),
            _ => None,
        }
    }
}

impl std::ops::Div for Integer {
    type Output = Option<Self>;

    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Integer::Field(lhs), Integer::Field(rhs)) => lhs.checked_div(&rhs).map(Integer::Field),
            (Integer::U8(lhs), Integer::U8(rhs)) => lhs.checked_div(rhs).map(Integer::U8),
            (Integer::U16(lhs), Integer::U16(rhs)) => lhs.checked_div(rhs).map(Integer::U16),
            (Integer::U32(lhs), Integer::U32(rhs)) => lhs.checked_div(rhs).map(Integer::U32),
            (Integer::U64(lhs), Integer::U64(rhs)) => lhs.checked_div(rhs).map(Integer::U64),
            (Integer::U128(lhs), Integer::U128(rhs)) => lhs.checked_div(rhs).map(Integer::U128),
            (Integer::I8(lhs), Integer::I8(rhs)) => lhs.checked_div(rhs).map(Integer::I8),
            (Integer::I16(lhs), Integer::I16(rhs)) => lhs.checked_div(rhs).map(Integer::I16),
            (Integer::I32(lhs), Integer::I32(rhs)) => lhs.checked_div(rhs).map(Integer::I32),
            (Integer::I64(lhs), Integer::I64(rhs)) => lhs.checked_div(rhs).map(Integer::I64),
            _ => None,
        }
    }
}

impl std::ops::Rem for Integer {
    type Output = Option<Self>;

    fn rem(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            // Fields do not support the remainder operation
            (Integer::Field(_), Integer::Field(_)) => None,
            (Integer::U8(lhs), Integer::U8(rhs)) => lhs.checked_rem(rhs).map(Integer::U8),
            (Integer::U16(lhs), Integer::U16(rhs)) => lhs.checked_rem(rhs).map(Integer::U16),
            (Integer::U32(lhs), Integer::U32(rhs)) => lhs.checked_rem(rhs).map(Integer::U32),
            (Integer::U64(lhs), Integer::U64(rhs)) => lhs.checked_rem(rhs).map(Integer::U64),
            (Integer::U128(lhs), Integer::U128(rhs)) => lhs.checked_rem(rhs).map(Integer::U128),
            (Integer::I8(lhs), Integer::I8(rhs)) => lhs.checked_rem(rhs).map(Integer::I8),
            (Integer::I16(lhs), Integer::I16(rhs)) => lhs.checked_rem(rhs).map(Integer::I16),
            (Integer::I32(lhs), Integer::I32(rhs)) => lhs.checked_rem(rhs).map(Integer::I32),
            (Integer::I64(lhs), Integer::I64(rhs)) => lhs.checked_rem(rhs).map(Integer::I64),
            _ => None,
        }
    }
}

impl Integer {
    /// `self < rhs`
    /// Returns `None` when the integer variants do not match, and for `Field` operands:
    /// fields have no ordering (their canonical representatives encode `-k` as `p - k`,
    /// which would invert signed intuition), matching the elaborator's rejection of
    /// `<`/`<=`/`>`/`>=` on `Field`.
    pub fn lt(&self, rhs: &Self) -> Option<bool> {
        match (self, rhs) {
            (Integer::U8(lhs), Integer::U8(rhs)) => Some(lhs < rhs),
            (Integer::U16(lhs), Integer::U16(rhs)) => Some(lhs < rhs),
            (Integer::U32(lhs), Integer::U32(rhs)) => Some(lhs < rhs),
            (Integer::U64(lhs), Integer::U64(rhs)) => Some(lhs < rhs),
            (Integer::U128(lhs), Integer::U128(rhs)) => Some(lhs < rhs),
            (Integer::I8(lhs), Integer::I8(rhs)) => Some(lhs < rhs),
            (Integer::I16(lhs), Integer::I16(rhs)) => Some(lhs < rhs),
            (Integer::I32(lhs), Integer::I32(rhs)) => Some(lhs < rhs),
            (Integer::I64(lhs), Integer::I64(rhs)) => Some(lhs < rhs),
            _ => None,
        }
    }

    /// `self <= rhs`
    /// Returns `None` when the integer variants do not match, and for `Field` operands:
    /// fields have no ordering (their canonical representatives encode `-k` as `p - k`,
    /// which would invert signed intuition), matching the elaborator's rejection of
    /// `<`/`<=`/`>`/`>=` on `Field`.
    pub fn lte(&self, rhs: &Self) -> Option<bool> {
        match (self, rhs) {
            (Integer::U8(lhs), Integer::U8(rhs)) => Some(lhs <= rhs),
            (Integer::U16(lhs), Integer::U16(rhs)) => Some(lhs <= rhs),
            (Integer::U32(lhs), Integer::U32(rhs)) => Some(lhs <= rhs),
            (Integer::U64(lhs), Integer::U64(rhs)) => Some(lhs <= rhs),
            (Integer::U128(lhs), Integer::U128(rhs)) => Some(lhs <= rhs),
            (Integer::I8(lhs), Integer::I8(rhs)) => Some(lhs <= rhs),
            (Integer::I16(lhs), Integer::I16(rhs)) => Some(lhs <= rhs),
            (Integer::I32(lhs), Integer::I32(rhs)) => Some(lhs <= rhs),
            (Integer::I64(lhs), Integer::I64(rhs)) => Some(lhs <= rhs),
            _ => None,
        }
    }
}

impl std::ops::Neg for Integer {
    type Output = Option<Self>;

    fn neg(self) -> Self::Output {
        match self {
            Integer::Field(rhs) => Some(Integer::Field(-rhs)),
            Integer::U8(_) => None,
            Integer::U16(_) => None,
            Integer::U32(_) => None,
            Integer::U64(_) => None,
            Integer::U128(_) => None,
            Integer::I8(rhs) => rhs.checked_neg().map(Integer::I8),
            Integer::I16(rhs) => rhs.checked_neg().map(Integer::I16),
            Integer::I32(rhs) => rhs.checked_neg().map(Integer::I32),
            Integer::I64(rhs) => rhs.checked_neg().map(Integer::I64),
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

    // === Proptests: Integer arithmetic matches Rust checked arithmetic ===

    proptest! {
        #[test]
        fn i8_add_matches_rust(a: i8, b: i8) {
            assert_eq!(Integer::I8(a) + Integer::I8(b), a.checked_add(b).map(Integer::I8));
        }

        #[test]
        fn i8_sub_matches_rust(a: i8, b: i8) {
            assert_eq!(Integer::I8(a) - Integer::I8(b), a.checked_sub(b).map(Integer::I8));
        }

        #[test]
        fn i8_mul_matches_rust(a: i8, b: i8) {
            assert_eq!(Integer::I8(a) * Integer::I8(b), a.checked_mul(b).map(Integer::I8));
        }

        #[test]
        fn i8_div_matches_rust(a: i8, b: i8) {
            assert_eq!(Integer::I8(a) / Integer::I8(b), a.checked_div(b).map(Integer::I8));
        }

        #[test]
        fn i8_rem_matches_rust(a: i8, b: i8) {
            assert_eq!(Integer::I8(a) % Integer::I8(b), a.checked_rem(b).map(Integer::I8));
        }

        #[test]
        fn i8_neg_matches_rust(a: i8) {
            assert_eq!(-Integer::I8(a), a.checked_neg().map(Integer::I8));
        }

        #[test]
        fn i8_lt_matches_rust(a: i8, b: i8) {
            assert_eq!(Integer::I8(a).lt(&Integer::I8(b)), Some(a < b));
        }

        #[test]
        fn i8_lte_matches_rust(a: i8, b: i8) {
            assert_eq!(Integer::I8(a).lte(&Integer::I8(b)), Some(a <= b));
        }

        #[test]
        fn i32_add_matches_rust(a: i32, b: i32) {
            assert_eq!(Integer::I32(a) + Integer::I32(b), a.checked_add(b).map(Integer::I32));
        }

        #[test]
        fn i32_sub_matches_rust(a: i32, b: i32) {
            assert_eq!(Integer::I32(a) - Integer::I32(b), a.checked_sub(b).map(Integer::I32));
        }

        #[test]
        fn i32_mul_matches_rust(a: i32, b: i32) {
            assert_eq!(Integer::I32(a) * Integer::I32(b), a.checked_mul(b).map(Integer::I32));
        }

        #[test]
        fn i32_div_matches_rust(a: i32, b: i32) {
            assert_eq!(Integer::I32(a) / Integer::I32(b), a.checked_div(b).map(Integer::I32));
        }

        #[test]
        fn u8_add_matches_rust(a: u8, b: u8) {
            assert_eq!(Integer::U8(a) + Integer::U8(b), a.checked_add(b).map(Integer::U8));
        }

        #[test]
        fn u8_sub_matches_rust(a: u8, b: u8) {
            assert_eq!(Integer::U8(a) - Integer::U8(b), a.checked_sub(b).map(Integer::U8));
        }

        #[test]
        fn u8_mul_matches_rust(a: u8, b: u8) {
            assert_eq!(Integer::U8(a) * Integer::U8(b), a.checked_mul(b).map(Integer::U8));
        }

        #[test]
        fn u8_neg_always_none(a: u8) {
            assert_eq!(-Integer::U8(a), None);
        }

        #[test]
        fn i8_is_negative_matches_rust(a: i8) {
            assert_eq!(Integer::I8(a).is_negative(), a < 0);
        }

        #[test]
        fn u8_is_negative_always_false(a: u8) {
            assert!(!Integer::U8(a).is_negative());
        }

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
            let integer = Integer::I8(a);
            let typ = Type::Integer(Signedness::Signed, IntegerBitSize::Eight);
            assert_eq!(Integer::try_from_field(encoded(&integer), &typ), Some(integer));
        }

        #[test]
        fn i16_try_from_field_roundtrips(a: i16) {
            let integer = Integer::I16(a);
            let typ = Type::Integer(Signedness::Signed, IntegerBitSize::Sixteen);
            assert_eq!(Integer::try_from_field(encoded(&integer), &typ), Some(integer));
        }

        #[test]
        fn i32_try_from_field_roundtrips(a: i32) {
            let integer = Integer::I32(a);
            let typ = Type::Integer(Signedness::Signed, IntegerBitSize::ThirtyTwo);
            assert_eq!(Integer::try_from_field(encoded(&integer), &typ), Some(integer));
        }

        #[test]
        fn i64_try_from_field_roundtrips(a: i64) {
            let integer = Integer::I64(a);
            let typ = Type::Integer(Signedness::Signed, IntegerBitSize::SixtyFour);
            assert_eq!(Integer::try_from_field(encoded(&integer), &typ), Some(integer));
        }

        #[test]
        fn u8_try_from_field_roundtrips(a: u8) {
            let integer = Integer::U8(a);
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

    // === Type mismatch returns None ===

    #[test]
    fn type_mismatch_returns_none() {
        let a = Integer::I8(1);
        let b = Integer::I16(1);
        assert_eq!(a.clone() + b.clone(), None);
        assert_eq!(a.clone() - b.clone(), None);
        assert_eq!(a.clone() * b.clone(), None);
        assert_eq!(a.clone() / b.clone(), None);
        assert_eq!(a.lt(&b), None);

        assert_eq!(Integer::U8(1) + Integer::I8(1), None);
    }

    // === Field-specific tests (not equivalent to Rust arithmetic) ===

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
        let neg_one = Integer::Field(encoded(&Integer::I64(-1)));
        let zero = Integer::Field(FieldValue::zero(FieldId::linked()));
        assert_eq!(neg_one.lt(&zero), None);
        assert_eq!(zero.lt(&neg_one), None);
    }

    #[test]
    fn field_lte_is_unordered() {
        let neg_one = Integer::Field(encoded(&Integer::I64(-1)));
        let zero = Integer::Field(FieldValue::zero(FieldId::linked()));
        assert_eq!(neg_one.lte(&zero), None);
        assert_eq!(zero.lte(&neg_one), None);
    }

    // === BigInt conversions ===

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
            let expected = i8::try_from(a).ok().map(Integer::I8);
            assert_eq!(Integer::try_from_bigint(&value, &typ, FieldId::linked()), expected);
        }

        #[test]
        fn try_from_bigint_matches_rust_conversion_for_u8(a: i128) {
            let value = BigInt::from(a);
            let typ = Type::Integer(Signedness::Unsigned, IntegerBitSize::Eight);
            let expected = u8::try_from(a).ok().map(Integer::U8);
            assert_eq!(Integer::try_from_bigint(&value, &typ, FieldId::linked()), expected);
        }

        // Integer -> BigInt -> Integer round-trips through try_from_bigint
        #[test]
        fn to_bigint_roundtrips_for_i64(a: i64) {
            let integer = Integer::I64(a);
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

        assert_eq!(from(BigInt::from(-128), &i8_type), Some(Integer::I8(-128)));
        assert_eq!(from(BigInt::from(127), &i8_type), Some(Integer::I8(127)));
        assert_eq!(from(BigInt::from(-129), &i8_type), None);
        assert_eq!(from(BigInt::from(128), &i8_type), None);

        let u128_type = Type::Integer(Unsigned, HundredTwentyEight);
        assert_eq!(from(BigInt::from(u128::MAX), &u128_type), Some(Integer::U128(u128::MAX)));
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
                (Type::Integer(Unsigned, Eight), u8::try_from(element).ok().map(Integer::U8)),
                (Type::Integer(Unsigned, Sixteen), u16::try_from(element).ok().map(Integer::U16)),
                (Type::Integer(Unsigned, ThirtyTwo), u32::try_from(element).ok().map(Integer::U32)),
                (Type::Integer(Unsigned, SixtyFour), u64::try_from(element).ok().map(Integer::U64)),
                (
                    Type::Integer(Unsigned, HundredTwentyEight),
                    u128::try_from(element).ok().map(Integer::U128),
                ),
                (Type::Integer(Signed, Eight), i8::try_from(element).ok().map(Integer::I8)),
                (Type::Integer(Signed, Sixteen), i16::try_from(element).ok().map(Integer::I16)),
                (Type::Integer(Signed, ThirtyTwo), i32::try_from(element).ok().map(Integer::I32)),
                (Type::Integer(Signed, SixtyFour), i64::try_from(element).ok().map(Integer::I64)),
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
            Some(Integer::I64(-9223372032559808513))
        );
        let u64_type = Type::Integer(Signedness::Unsigned, IntegerBitSize::SixtyFour);
        assert_eq!(
            Integer::try_from_field(value, &u64_type),
            Some(Integer::U64(1 << 63)),
            "an unsigned target reads the value itself"
        );
    }

    #[test]
    #[should_panic(expected = "ICE: value does not fit in the field")]
    fn bigint_to_field_panics_on_non_canonical_values() {
        let modulus = BigInt::from_biguint(Sign::Plus, FieldElement::modulus());
        bigint_to_field(&modulus);
    }
}
