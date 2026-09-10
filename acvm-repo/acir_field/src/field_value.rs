//! Field arithmetic with a field identity chosen at run time.

use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::ops::{Add, Mul, Neg, Sub};

use num_bigint::{BigInt, BigUint, Sign};

use crate::{AcirField, Bn254FieldElement, FieldConfig, FieldElement, FieldId};

const I128_SIGN_BOUNDARY: u128 = 1_u128 << 127;

/// A canonical representative in `[0, p)` and its field identity. Ordering compares representatives first.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldValue {
    value: BigUint,
    field: FieldId,
}

impl FieldValue {
    pub fn zero(field: FieldId) -> FieldValue {
        FieldValue { value: BigUint::ZERO, field }
    }

    pub fn one(field: FieldId) -> FieldValue {
        FieldValue { value: BigUint::from(1u8), field }
    }

    /// Reject values at or above the modulus.
    pub fn try_from_biguint(value: BigUint, field: FieldId) -> Option<FieldValue> {
        (value < *FieldConfig::new(field).modulus()).then_some(FieldValue { value, field })
    }

    /// Encode negative values by field negation; reject magnitudes at or above the modulus.
    pub fn try_from_bigint(value: &BigInt, field: FieldId) -> Option<FieldValue> {
        let magnitude = FieldValue::try_from_biguint(value.magnitude().clone(), field)?;
        Some(if value.sign() == Sign::Minus { -magnitude } else { magnitude })
    }

    /// Convert bn254 values for the blackbox solvers; reject other fields.
    pub fn to_bn254_element(&self) -> Option<Bn254FieldElement> {
        (self.field == FieldId::Bn254)
            .then(|| Bn254FieldElement::from_be_bytes_reduce(&self.value.to_bytes_be()))
    }

    /// Preserve the value and field of a linked backend element.
    pub fn from_linked_element(value: FieldElement) -> FieldValue {
        FieldValue { value: BigUint::from_bytes_be(&value.to_be_bytes()), field: FieldId::linked() }
    }

    pub fn from_bn254_element(value: Bn254FieldElement) -> FieldValue {
        FieldValue { value: BigUint::from_bytes_be(&value.to_be_bytes()), field: FieldId::Bn254 }
    }

    pub fn field(&self) -> FieldId {
        self.field
    }

    /// The canonical representative, in `[0, p)`.
    pub fn as_biguint(&self) -> &BigUint {
        &self.value
    }

    /// The canonical representative as a signed integer, never negative.
    pub fn to_bigint(&self) -> BigInt {
        BigInt::from(self.value.clone())
    }

    /// Choose the shorter decimal representation of `value` and `-(p - value)`, as [`Display`] does.
    pub fn to_signed_bigint(&self) -> BigInt {
        let negated = self.negated();
        if negated.to_string().len() < self.value.to_string().len() {
            -BigInt::from(negated)
        } else {
            BigInt::from(self.value.clone())
        }
    }

    pub fn is_zero(&self) -> bool {
        self.value == BigUint::ZERO
    }

    pub fn is_one(&self) -> bool {
        self.value == BigUint::from(1u8)
    }

    /// The bit length of the representative; zero needs no bits.
    pub fn num_bits(&self) -> u32 {
        u32::try_from(self.value.bits()).expect("a canonical value is no wider than its modulus")
    }

    /// Hexadecimal with a `0x` prefix, whole bytes and no leading zero bytes.
    pub fn to_short_hex(&self) -> String {
        format!("0x{}", hex::encode(self.value.to_bytes_be()))
    }

    /// The representative in hexadecimal at the full width of a serialized element of its field.
    pub fn to_hex(&self) -> String {
        let width = self.config().num_bytes() as usize * 2;
        format!("{:0width$x}", self.value)
    }

    pub fn try_to_u32(&self) -> Option<u32> {
        u32::try_from(&self.value).ok()
    }

    pub fn try_to_u64(&self) -> Option<u64> {
        u64::try_from(&self.value).ok()
    }

    pub fn try_into_u128(&self) -> Option<u128> {
        u128::try_from(&self.value).ok()
    }

    /// Whether the positive or field-negated representation fits in `i128`, including `i128::MIN`.
    pub fn fits_in_i128(&self) -> bool {
        let negated = self.negated();
        self.value.bits() <= 127
            || negated.bits() <= 127
            || negated == BigUint::from(I128_SIGN_BOUNDARY) % self.modulus()
    }

    /// The `i128` this element spells, or `None` if neither spelling fits one.
    pub fn try_into_i128(&self) -> Option<i128> {
        self.fits_in_i128().then(|| self.to_i128())
    }

    /// The shorter of the two spellings by bit length wins, ties reading positive.
    fn to_i128(&self) -> i128 {
        let negated = self.negated();
        if negated.bits() < self.value.bits() {
            // `as i128` reads 2^127 as `i128::MIN`, which only `wrapping_neg` maps to itself.
            (Self::to_u128(&negated) as i128).wrapping_neg()
        } else {
            Self::to_u128(&self.value) as i128
        }
    }

    fn to_u128(value: &BigUint) -> u128 {
        u128::try_from(value).expect("the spelling chosen by `fits_in_i128` is at most 2^127")
    }

    /// `self / rhs`, or `None` if `rhs` is zero.
    pub fn checked_div(&self, rhs: &FieldValue) -> Option<FieldValue> {
        let modulus = self.modulus_shared_with(rhs);
        (!rhs.is_zero())
            .then(|| FieldValue { value: &self.value * rhs.inverse() % modulus, field: self.field })
    }

    fn config(&self) -> FieldConfig {
        FieldConfig::new(self.field)
    }

    fn modulus(&self) -> &'static BigUint {
        self.config().modulus()
    }

    /// Mixed fields indicate a compiler bug.
    fn modulus_shared_with(&self, other: &FieldValue) -> &'static BigUint {
        assert_eq!(
            self.field, other.field,
            "ICE: a Field value of {} meets a value of {}",
            self.field, other.field
        );
        self.modulus()
    }

    fn negated(&self) -> BigUint {
        if self.is_zero() { BigUint::ZERO } else { self.modulus() - &self.value }
    }

    /// The multiplicative inverse, by Fermat's little theorem; zero inverts to zero.
    fn inverse(&self) -> BigUint {
        self.value.modpow(&(self.modulus() - 2u8), self.modulus())
    }
}

impl Hash for FieldValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Preserve the hashes exposed by comptime reflection in existing bn254 programs.
        if let Some(value) = self.to_bn254_element() {
            value.hash(state);
        } else {
            self.value.hash(state);
            self.field.hash(state);
        }
    }
}

impl Display for FieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_signed_bigint())
    }
}

impl std::fmt::Debug for FieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Neg for FieldValue {
    type Output = FieldValue;

    fn neg(self) -> FieldValue {
        FieldValue { value: self.negated(), field: self.field }
    }
}

impl Add for FieldValue {
    type Output = FieldValue;

    fn add(self, rhs: FieldValue) -> FieldValue {
        let modulus = self.modulus_shared_with(&rhs);
        FieldValue { value: (self.value + rhs.value) % modulus, field: self.field }
    }
}

impl Sub for FieldValue {
    type Output = FieldValue;

    fn sub(self, rhs: FieldValue) -> FieldValue {
        self + (-rhs)
    }
}

impl Mul for FieldValue {
    type Output = FieldValue;

    fn mul(self, rhs: FieldValue) -> FieldValue {
        let modulus = self.modulus_shared_with(&rhs);
        FieldValue { value: self.value * rhs.value % modulus, field: self.field }
    }
}

#[cfg(test)]
mod tests {
    use num_bigint::{BigInt, BigUint};
    use proptest::prelude::*;

    use crate::{AcirField, FieldConfig, FieldElement, FieldId, FieldValue};

    fn modulus(field: FieldId) -> &'static BigUint {
        FieldConfig::new(field).modulus()
    }

    fn small(field: FieldId, value: u64) -> FieldValue {
        FieldValue::try_from_biguint(BigUint::from(value), field).expect("below every modulus")
    }

    #[test]
    fn a_value_is_canonical_or_refused() {
        for field in FieldId::ALL {
            let p = modulus(field);
            assert_eq!(FieldValue::try_from_biguint(p.clone(), field), None, "{field}");
            assert_eq!(FieldValue::try_from_biguint(p + 1u8, field), None, "{field}");

            let largest = FieldValue::try_from_biguint(p - 1u8, field).expect("p - 1 is canonical");
            assert_eq!(largest.as_biguint(), &(p - 1u8), "{field}");
            assert_eq!(largest.field(), field);
            assert_eq!(largest, -FieldValue::one(field), "{field}");
        }
    }

    #[test]
    fn a_negative_bigint_is_the_negation_of_its_magnitude() {
        for field in FieldId::ALL {
            let p = BigInt::from(modulus(field).clone());

            assert_eq!(
                FieldValue::try_from_bigint(&BigInt::from(-1), field),
                Some(-FieldValue::one(field)),
                "{field}"
            );
            assert_eq!(
                FieldValue::try_from_bigint(&BigInt::ZERO, field),
                Some(FieldValue::zero(field)),
                "{field}"
            );

            assert_eq!(FieldValue::try_from_bigint(&p, field), None, "{field}");
            assert_eq!(FieldValue::try_from_bigint(&-p.clone(), field), None, "{field}");
            assert_eq!(FieldValue::try_from_bigint(&(p + 1), field), None, "{field}");
        }
    }

    #[test]
    fn arithmetic_wraps_at_each_modulus() {
        for field in FieldId::ALL {
            let (zero, one) = (FieldValue::zero(field), FieldValue::one(field));
            let largest = FieldValue::try_from_biguint(modulus(field) - 1u8, field).unwrap();

            assert!(zero.is_zero() && one.is_one(), "{field}");
            assert_eq!(largest.clone() + one.clone(), zero, "{field}");
            assert_eq!(zero.clone() - one.clone(), largest, "{field}");
            assert_eq!(-zero.clone(), zero, "{field}");
            assert_eq!(one.clone() * largest.clone(), largest, "{field}");
            assert_eq!(largest.clone() * largest.clone(), one, "{field}");

            let two = small(field, 2);
            let half = one.checked_div(&two).expect("2 is invertible");
            assert_eq!(two * half, one, "{field}");
            assert_eq!(one.checked_div(&zero), None, "{field}");
        }
    }

    #[test]
    fn goldilocks_reduces_two_to_the_sixty_fourth() {
        let field = FieldId::Goldilocks;
        let largest = FieldValue::try_from_biguint(modulus(field) - 1u8, field).unwrap();
        let two_to_the_64 = largest + small(field, 1 << 32);
        assert_eq!(two_to_the_64.as_biguint(), &BigUint::from((1u64 << 32) - 1));
    }

    #[test]
    #[should_panic(expected = "ICE: a Field value of goldilocks meets a value of bn254")]
    fn mixing_two_fields_is_an_internal_error() {
        let _ = small(FieldId::Goldilocks, 1) + small(FieldId::Bn254, 1);
    }

    // Include decimal and binary boundaries that random samples are unlikely to reach.
    fn boundary_values(field: FieldId) -> Vec<BigUint> {
        let p = modulus(field);
        let bits = FieldConfig::new(field).num_bits();
        let one = BigUint::from(1u8);

        let mut values =
            vec![BigUint::ZERO, one.clone(), BigUint::from(2u8), p - 1u8, p - 2u8, p / 2u8];
        values.push(p / 2u8 + 1u8);

        for power in [1u32, 30, 31, 32, 62, 63, 64, 126, 127, 128, 253, 254] {
            if power < bits {
                let value = one.clone() << power;
                values.push(p - &value);
                values.push(value);
            }
        }
        for digits in [1u32, 9, 18, 19, 37, 38, 39, 76] {
            let value = BigUint::from(10u8).pow(digits);
            if value < *p {
                values.push(p - &value);
                values.push(value);
            }
        }
        values
    }

    fn linked(value: FieldElement) -> FieldValue {
        FieldValue::from_linked_element(value)
    }

    fn signed_spelling(element: FieldElement) -> BigInt {
        let positive = BigInt::from(BigUint::from_bytes_be(&element.to_be_bytes()));
        let negated = BigInt::from(BigUint::from_bytes_be(&(-element).to_be_bytes()));
        if negated.to_string().len() < positive.to_string().len() { -negated } else { positive }
    }

    fn agrees_with(element: FieldElement) {
        let value = linked(element);

        if value.field() == FieldId::Bn254 {
            use std::hash::{DefaultHasher, Hash, Hasher};
            let mut value_hash = DefaultHasher::new();
            let mut element_hash = DefaultHasher::new();
            value.hash(&mut value_hash);
            element.hash(&mut element_hash);
            assert_eq!(value_hash.finish(), element_hash.finish(), "hash of {element}");
        }

        assert_eq!(value.num_bits(), element.num_bits(), "{element}");
        assert_eq!(value.is_zero(), element.is_zero(), "{element}");
        assert_eq!(value.is_one(), element.is_one(), "{element}");
        assert_eq!(value.to_short_hex(), element.to_short_hex(), "{element}");
        assert_eq!(value.to_hex(), element.to_hex(), "{element}");
        assert_eq!(value.to_string(), element.to_string(), "{element}");
        assert_eq!(format!("{value:?}"), format!("{element:?}"), "{element}");
        assert_eq!(value.to_signed_bigint(), signed_spelling(element), "{element}");
        assert_eq!(value.try_to_u32(), element.try_to_u32(), "{element}");
        assert_eq!(value.try_to_u64(), element.try_to_u64(), "{element}");
        assert_eq!(value.try_into_u128(), element.try_into_u128(), "{element}");
        assert_eq!(value.fits_in_i128(), element.fits_in_i128(), "{element}");
        assert_eq!(value.try_into_i128(), element.try_into_i128(), "{element}");
        assert_eq!(-value, linked(-element), "{element}");
    }

    fn pair_agrees_with(a: FieldElement, b: FieldElement) {
        let (x, y) = (linked(a), linked(b));

        assert_eq!(x.clone() + y.clone(), linked(a + b), "{a} + {b}");
        assert_eq!(x.clone() - y.clone(), linked(a - b), "{a} - {b}");
        assert_eq!(x.clone() * y.clone(), linked(a * b), "{a} * {b}");
        let quotient = if b.is_zero() { None } else { Some(linked(a / b)) };
        assert_eq!(x.checked_div(&y), quotient, "{a} / {b}");
    }

    #[test]
    fn every_boundary_value_agrees_with_the_linked_field_element() {
        let values = boundary_values(FieldId::linked());
        for value in &values {
            agrees_with(FieldElement::from_be_bytes_reduce(&value.to_bytes_be()));
        }
        for a in &values {
            for b in &values {
                pair_agrees_with(
                    FieldElement::from_be_bytes_reduce(&a.to_bytes_be()),
                    FieldElement::from_be_bytes_reduce(&b.to_bytes_be()),
                );
            }
        }
    }

    proptest! {
        #[test]
        fn a_random_value_agrees_with_the_linked_field_element(
            bytes in prop::collection::vec(any::<u8>(), 1..=32)
        ) {
            agrees_with(FieldElement::from_be_bytes_reduce(&bytes));
        }

        #[test]
        fn a_random_pair_agrees_with_the_linked_field_elements(
            a in prop::collection::vec(any::<u8>(), 1..=32),
            b in prop::collection::vec(any::<u8>(), 1..=32),
        ) {
            pair_agrees_with(
                FieldElement::from_be_bytes_reduce(&a),
                FieldElement::from_be_bytes_reduce(&b),
            );
        }
    }
}
