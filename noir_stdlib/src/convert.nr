use crate::field::MODULUS_BITS;
use crate::option::Option;

/// Create a new `Self` value from the given `T` value
// docs:start:from-trait
pub trait From<T> {
    fn from(input: T) -> Self;
}
// docs:end:from-trait

impl<T> From<T> for T {
    fn from(input: T) -> T {
        input
    }
}

/// Convert a `Self` value into a `T` value
// docs:start:into-trait
pub trait Into<T> {
    fn into(self) -> T;
}

impl<T, U> Into<T> for U
where
    T: From<U>,
{
    fn into(self) -> T {
        T::from(self)
    }
}
// docs:end:into-trait

// docs:start:from-impls
// Unsigned integers

impl From<u8> for u16 {
    fn from(value: u8) -> u16 {
        value as u16
    }
}

impl From<u8> for u32 {
    fn from(value: u8) -> u32 {
        value as u32
    }
}

impl From<u16> for u32 {
    fn from(value: u16) -> u32 {
        value as u32
    }
}

impl From<u8> for u64 {
    fn from(value: u8) -> u64 {
        value as u64
    }
}

impl From<u16> for u64 {
    fn from(value: u16) -> u64 {
        value as u64
    }
}

impl From<u32> for u64 {
    fn from(value: u32) -> u64 {
        value as u64
    }
}

impl From<u8> for u128 {
    fn from(value: u8) -> u128 {
        value as u128
    }
}

impl From<u16> for u128 {
    fn from(value: u16) -> u128 {
        value as u128
    }
}

impl From<u32> for u128 {
    fn from(value: u32) -> u128 {
        value as u128
    }
}
impl From<u64> for u128 {
    fn from(value: u64) -> u128 {
        value as u128
    }
}

impl From<u8> for Field {
    fn from(value: u8) -> Field {
        value as Field
    }
}

impl From<u16> for Field {
    fn from(value: u16) -> Field {
        value as Field
    }
}

impl From<u32> for Field {
    fn from(value: u32) -> Field {
        value as Field
    }
}
// These conversions require every source value to fit in the field.
#[field(not(goldilocks))]
impl From<u64> for Field {
    fn from(value: u64) -> Field {
        value as Field
    }
}

#[field(not(goldilocks))]
impl From<u128> for Field {
    fn from(value: u128) -> Field {
        value as Field
    }
}

// Signed integers

impl From<i8> for i16 {
    fn from(value: i8) -> i16 {
        value as i16
    }
}

impl From<i8> for i32 {
    fn from(value: i8) -> i32 {
        value as i32
    }
}

impl From<i16> for i32 {
    fn from(value: i16) -> i32 {
        value as i32
    }
}

impl From<i8> for i64 {
    fn from(value: i8) -> i64 {
        value as i64
    }
}

impl From<i16> for i64 {
    fn from(value: i16) -> i64 {
        value as i64
    }
}

impl From<i32> for i64 {
    fn from(value: i32) -> i64 {
        value as i64
    }
}

// Booleans
impl From<bool> for u8 {
    fn from(value: bool) -> u8 {
        value as u8
    }
}
impl From<bool> for u16 {
    fn from(value: bool) -> u16 {
        value as u16
    }
}
impl From<bool> for u32 {
    fn from(value: bool) -> u32 {
        value as u32
    }
}
impl From<bool> for u64 {
    fn from(value: bool) -> u64 {
        value as u64
    }
}
impl From<bool> for u128 {
    fn from(value: bool) -> u128 {
        value as u128
    }
}
impl From<bool> for i8 {
    fn from(value: bool) -> i8 {
        value as i8
    }
}
impl From<bool> for i16 {
    fn from(value: bool) -> i16 {
        value as i16
    }
}
impl From<bool> for i32 {
    fn from(value: bool) -> i32 {
        value as i32
    }
}
impl From<bool> for i64 {
    fn from(value: bool) -> i64 {
        value as i64
    }
}
impl From<bool> for Field {
    fn from(value: bool) -> Field {
        value as Field
    }
}
// docs:end:from-impls

/// Create a new `Self` value from the given `T` value, wrapping a value that does not fit
// docs:start:unsafe-from-trait
pub trait UnsafeFrom<T> {
    fn unsafe_from(input: T) -> Self;
}
// docs:end:unsafe-from-trait

// docs:start:unsafe-from-impls
impl<let N: u32> UnsafeFrom<u<N>> for Field {
    fn unsafe_from(input: u<N>) -> Field {
        input as Field
    }
}

// A signed integer converts as its bit pattern, so `-1_i8` is `255`.
impl<let N: u32> UnsafeFrom<i<N>> for Field {
    fn unsafe_from(input: i<N>) -> Field {
        input as u::<N> as Field
    }
}

impl<let N: u32> UnsafeFrom<Field> for u<N> {
    fn unsafe_from(input: Field) -> u<N> {
        input as Self
    }
}

impl<let N: u32> UnsafeFrom<Field> for i<N> {
    fn unsafe_from(input: Field) -> i<N> {
        input as Self
    }
}
// docs:end:unsafe-from-impls

/// Create a new `Self` value from the given `T` value, or `None` if the value does not fit
// docs:start:try-from-trait
pub trait TryFrom<T> {
    fn try_from(input: T) -> Option<Self>;
}
// docs:end:try-from-trait

// docs:start:try-from-impls
// A value fits when the wrapping conversion brings it back unchanged, which every value does
// where the target is the wider of the two.
impl<let N: u32> TryFrom<u<N>> for Field {
    fn try_from(input: u<N>) -> Option<Field> {
        let output = Field::unsafe_from(input);
        let back: u<N> = UnsafeFrom::unsafe_from(output);
        if (N < MODULUS_BITS) | (back == input) {
            Option::some(output)
        } else {
            Option::none()
        }
    }
}

impl<let N: u32> TryFrom<i<N>> for Field {
    fn try_from(input: i<N>) -> Option<Field> {
        if input < 0 {
            Option::none()
        } else {
            Field::try_from(input as u::<N>)
        }
    }
}

impl<let N: u32> TryFrom<Field> for u<N> {
    fn try_from(input: Field) -> Option<u<N>> {
        let output: u<N> = UnsafeFrom::unsafe_from(input);
        if (N >= MODULUS_BITS) | (Field::unsafe_from(output) == input) {
            Option::some(output)
        } else {
            Option::none()
        }
    }
}

impl<let N: u32> TryFrom<Field> for i<N> {
    fn try_from(input: Field) -> Option<i<N>> {
        let output: i<N> = UnsafeFrom::unsafe_from(input);
        if (N > MODULUS_BITS) | ((output >= 0) & (Field::unsafe_from(output) == input)) {
            Option::some(output)
        } else {
            Option::none()
        }
    }
}
// docs:end:try-from-impls

/// A generic interface for casting between primitive types,
/// equivalent of using the `as` keyword between values.
///
/// # Example
///
/// ```
/// let x: u32 = 1234567890;
/// let y: u8 = x as u8;
/// let z: u8 = x.as_();
/// assert_eq(y, z);
/// ```
pub trait AsPrimitive<T> {
    /// The equivalent of doing `self as T`.
    fn as_(self) -> T;
}

#[generate_as_primitive_impls]
comptime fn generate_as_primitive_impls(_: FunctionDefinition) -> Quoted {
    let types = [
        quote { bool },
        quote { u8 },
        quote { u16 },
        quote { u32 },
        quote { u64 },
        quote { u128 },
        quote { i8 },
        quote { i16 },
        quote { i32 },
        quote { i64 },
    ];

    let mut impls = [].as_vector();
    for type1 in types {
        for type2 in types {
            let body = if type1 == type2 {
                quote { self }
            } else if type1 == quote { bool } {
                quote { self != 0 }
            } else {
                quote { self as $type1 }
            };

            impls = impls.push_back(
                quote {
                    impl AsPrimitive<$type1> for $type2 {
                        fn as_(self) -> $type1 {
                            $body
                        }
                    }
                },
            );
        }
    }

    impls.join(quote {})
}

mod tests {
    use crate::meta::unquote;
    use super::{From, Into, TryFrom, UnsafeFrom};

    #[test]
    fn from_identity() {
        let x: u32 = 42;
        assert_eq(u32::from(x), 42);
    }

    #[test]
    fn from_unsigned_widening() {
        comptime {
            let pairs = @[
                (quote { u8 }, quote { u16 }),
                (quote { u8 }, quote { u32 }),
                (quote { u8 }, quote { u64 }),
                (quote { u8 }, quote { u128 }),
                (quote { u16 }, quote { u32 }),
                (quote { u16 }, quote { u64 }),
                (quote { u16 }, quote { u128 }),
                (quote { u32 }, quote { u64 }),
                (quote { u32 }, quote { u128 }),
                (quote { u64 }, quote { u128 }),
            ];
            for pair in pairs {
                let source = pair.0;
                let target = pair.1;
                let src_zero = f"0_{source}".quoted_contents();
                let src_value = f"123_{source}".quoted_contents();
                let dst_zero = f"0_{target}".quoted_contents();
                let dst_value = f"123_{target}".quoted_contents();
                unquote!(
                    quote {
                        assert_eq($target::from($src_zero), $dst_zero);
                        assert_eq($target::from($src_value), $dst_value);
                    },
                );
            }
        }
    }

    #[test]
    fn from_unsigned_to_field() {
        comptime {
            // `Field: From<T>` exists only for types whose every value lies below the modulus.
            let field_bits = crate::field::modulus_num_bits();
            let sources: [(Quoted, u64); 5] = [
                (quote { u8 }, 8),
                (quote { u16 }, 16),
                (quote { u32 }, 32),
                (quote { u64 }, 64),
                (quote { u128 }, 128),
            ];
            for pair in sources {
                let (source, bits) = pair;
                if bits < field_bits {
                    let src_zero = f"0_{source}".quoted_contents();
                    let src_value = f"123_{source}".quoted_contents();
                    unquote!(
                        quote {
                            assert_eq(Field::from($src_zero), 0);
                            assert_eq(Field::from($src_value), 123);
                        },
                    );
                }
            }
        }
    }

    #[test]
    fn from_signed_widening() {
        comptime {
            let pairs = @[
                (quote { i8 }, quote { i16 }),
                (quote { i8 }, quote { i32 }),
                (quote { i8 }, quote { i64 }),
                (quote { i16 }, quote { i32 }),
                (quote { i16 }, quote { i64 }),
                (quote { i32 }, quote { i64 }),
            ];
            for pair in pairs {
                let source = pair.0;
                let target = pair.1;
                let src_pos = f"42_{source}".quoted_contents();
                let dst_pos = f"42_{target}".quoted_contents();
                let src_neg = f"-42_{source}".quoted_contents();
                let dst_neg = f"-42_{target}".quoted_contents();
                unquote!(
                    quote {
                        assert_eq($target::from($src_pos), $dst_pos);
                        assert_eq($target::from($src_neg), $dst_neg);
                    },
                );
            }
        }
    }

    #[test]
    fn from_bool() {
        comptime {
            for target in @[
                quote { u8 },
                quote { u16 },
                quote { u32 },
                quote { u64 },
                quote { u128 },
                quote { i8 },
                quote { i16 },
                quote { i32 },
                quote { i64 },
                quote { Field },
            ] {
                let one = f"1_{target}".quoted_contents();
                let zero = f"0_{target}".quoted_contents();
                unquote!(
                    quote {
                        assert_eq($target::from(true), $one);
                        assert_eq($target::from(false), $zero);
                    },
                );
            }
        }
    }

    #[test]
    fn unsafe_from_wraps() {
        let small: u128 = 0x1234;
        assert_eq(Field::unsafe_from(small), 0x1234);
        let negative: i8 = -1;
        assert_eq(Field::unsafe_from(negative), 255);

        let element: Field = 0x1234;
        assert_eq(u8::unsafe_from(element), 0x34);
        assert_eq(u128::unsafe_from(element), 0x1234);
        let byte: Field = 255;
        assert_eq(i8::unsafe_from(byte), -1);
    }

    #[test]
    fn try_from_is_none_when_the_value_does_not_fit() {
        let small: u32 = 300;
        let element = Field::try_from(small).unwrap();
        assert_eq(element, 300);
        assert_eq(u32::try_from(element).unwrap(), 300);
        assert_eq(u16::try_from(element).unwrap(), 300);
        assert(u8::try_from(element).is_none());

        let negative: i8 = -5;
        assert(Field::try_from(negative).is_none());
        let most: Field = 127;
        assert_eq(i8::try_from(most).unwrap(), 127);
        let past: Field = 128;
        assert(i8::try_from(past).is_none());
    }

    #[test]
    fn into_widens_unsigned() {
        let value: u8 = 7;
        let widened: u32 = value.into();
        assert_eq(widened, 7);
    }
}
