use num_bigint::{BigInt, BigUint};
use num_traits::{Num, One, Zero};
use std::collections::{BTreeMap, HashSet};
use thiserror::Error;

use acvm::{AcirField, FieldElement};
use itertools::Itertools;
use serde::Serialize;

use crate::errors::InputParserError;
use crate::{Abi, AbiType};

pub mod json;
mod toml;

/// This is what all formats eventually transform into
/// For example, a toml file will parse into `TomlTypes`
/// and those `TomlTypes` will be mapped to Value
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum InputValue {
    Field(FieldElement),
    String(String),
    Vec(Vec<InputValue>),
    Struct(BTreeMap<String, InputValue>),
}

#[derive(Debug, Error)]
pub enum InputTypecheckingError {
    #[error("Value {value:?} does not fall within range of allowable values for a {typ:?}")]
    OutsideOfValidRange { path: String, typ: AbiType, value: InputValue },
    #[error(
        "Type {typ:?} is expected to have length {expected_length} but value {value:?} has length {actual_length}"
    )]
    LengthMismatch {
        path: String,
        typ: AbiType,
        value: InputValue,
        expected_length: usize,
        actual_length: usize,
    },
    #[error(
        "Could not find value for required field `{expected_field}`. Found values for fields {found_fields:?}"
    )]
    MissingField { path: String, expected_field: String, found_fields: Vec<String> },
    #[error(
        "Additional unexpected field was provided for type {typ:?}. Found field named `{extra_field}`"
    )]
    UnexpectedField { path: String, typ: AbiType, extra_field: String },
    #[error("Type {typ:?} and value {value:?} do not match")]
    IncompatibleTypes { path: String, typ: AbiType, value: InputValue },
}

impl InputTypecheckingError {
    pub(crate) fn path(&self) -> &str {
        match self {
            InputTypecheckingError::OutsideOfValidRange { path, .. }
            | InputTypecheckingError::LengthMismatch { path, .. }
            | InputTypecheckingError::MissingField { path, .. }
            | InputTypecheckingError::UnexpectedField { path, .. }
            | InputTypecheckingError::IncompatibleTypes { path, .. } => path,
        }
    }
}

impl InputValue {
    /// Checks whether the ABI type matches the `InputValue` type
    pub(crate) fn find_type_mismatch(
        &self,
        abi_param: &AbiType,
        path: String,
    ) -> Result<(), InputTypecheckingError> {
        match (self, abi_param) {
            (InputValue::Field(_), AbiType::Field) => Ok(()),
            (InputValue::Field(field_element), AbiType::Integer { width, .. }) => {
                if field_element.num_bits() <= *width {
                    Ok(())
                } else {
                    Err(InputTypecheckingError::OutsideOfValidRange {
                        path,
                        typ: abi_param.clone(),
                        value: self.clone(),
                    })
                }
            }
            (InputValue::Field(field_element), AbiType::Boolean) => {
                if field_element.is_one() || field_element.is_zero() {
                    Ok(())
                } else {
                    Err(InputTypecheckingError::OutsideOfValidRange {
                        path,
                        typ: abi_param.clone(),
                        value: self.clone(),
                    })
                }
            }

            (InputValue::Vec(array_elements), AbiType::Array { length, typ, .. }) => {
                if array_elements.len() != *length as usize {
                    return Err(InputTypecheckingError::LengthMismatch {
                        path,
                        typ: abi_param.clone(),
                        value: self.clone(),
                        expected_length: *length as usize,
                        actual_length: array_elements.len(),
                    });
                }
                // Check that all of the array's elements' values match the ABI as well.
                for (i, element) in array_elements.iter().enumerate() {
                    let mut path = path.clone();
                    path.push_str(&format!("[{i}]"));

                    element.find_type_mismatch(typ, path)?;
                }
                Ok(())
            }

            (InputValue::String(string), AbiType::String { length }) => {
                if string.len() == *length as usize {
                    Ok(())
                } else {
                    Err(InputTypecheckingError::LengthMismatch {
                        path,
                        typ: abi_param.clone(),
                        value: self.clone(),
                        actual_length: string.len(),
                        expected_length: *length as usize,
                    })
                }
            }

            (InputValue::Struct(map), AbiType::Struct { fields, .. }) => {
                for (field_name, field_type) in fields {
                    if let Some(value) = map.get(field_name) {
                        let mut path = path.clone();
                        path.push_str(&format!(".{field_name}"));
                        value.find_type_mismatch(field_type, path)?;
                    } else {
                        return Err(InputTypecheckingError::MissingField {
                            path,
                            expected_field: field_name.clone(),
                            found_fields: map.keys().cloned().collect(),
                        });
                    }
                }

                if map.len() > fields.len() {
                    let expected_fields: HashSet<String> =
                        fields.iter().map(|(field, _)| field.clone()).collect();
                    let extra_field = map.keys().find(|&key| !expected_fields.contains(key)).cloned().expect("`map` is larger than the expected type's `fields` so it must contain an unexpected field");
                    return Err(InputTypecheckingError::UnexpectedField {
                        path,
                        typ: abi_param.clone(),
                        extra_field,
                    });
                }

                Ok(())
            }

            (InputValue::Vec(vec_elements), AbiType::Tuple { fields }) => {
                if vec_elements.len() != fields.len() {
                    return Err(InputTypecheckingError::LengthMismatch {
                        path,
                        typ: abi_param.clone(),
                        value: self.clone(),
                        actual_length: vec_elements.len(),
                        expected_length: fields.len(),
                    });
                }
                // Check that all of the array's elements' values match the ABI as well.
                for (i, (element, expected_typ)) in vec_elements.iter().zip_eq(fields).enumerate() {
                    let mut path = path.clone();
                    path.push_str(&format!(".{i}"));
                    element.find_type_mismatch(expected_typ, path)?;
                }
                Ok(())
            }

            // All other InputValue-AbiType combinations are fundamentally incompatible.
            _ => Err(InputTypecheckingError::IncompatibleTypes {
                path,
                typ: abi_param.clone(),
                value: self.clone(),
            }),
        }
    }

    /// Checks whether the ABI type matches the `InputValue` type.
    pub fn matches_abi(&self, abi_param: &AbiType) -> bool {
        self.find_type_mismatch(abi_param, String::new()).is_ok()
    }
}

/// The different formats that are supported when parsing
/// the initial witness values
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(test, derive(strum_macros::EnumIter))]
pub enum Format {
    Json,
    Toml,
}

impl Format {
    pub fn ext(&self) -> &'static str {
        match self {
            Format::Json => "json",
            Format::Toml => "toml",
        }
    }

    pub fn from_ext(ext: &str) -> Option<Self> {
        match ext {
            "json" => Some(Self::Json),
            "toml" => Some(Self::Toml),
            _ => None,
        }
    }
}

impl Format {
    pub fn parse(
        &self,
        input_string: &str,
        abi: &Abi,
    ) -> Result<BTreeMap<String, InputValue>, InputParserError> {
        match self {
            Format::Json => json::parse_json(input_string, abi),
            Format::Toml => toml::parse_toml(input_string, abi),
        }
    }

    pub fn serialize(
        &self,
        input_map: &BTreeMap<String, InputValue>,
        abi: &Abi,
    ) -> Result<String, InputParserError> {
        match self {
            Format::Json => json::serialize_to_json(input_map, abi),
            Format::Toml => toml::serialize_to_toml(input_map, abi),
        }
    }
}

#[cfg(test)]
mod serialization_tests {
    use std::collections::BTreeMap;

    use acvm::{AcirField, FieldElement};
    use strum::IntoEnumIterator;

    use crate::{
        Abi, AbiParameter, AbiReturnType, AbiType, AbiVisibility, MAIN_RETURN_NAME, Sign,
        input_parser::InputValue,
    };

    use super::Format;

    #[test]
    fn serialization_round_trip() {
        let abi = Abi {
            parameters: vec![
                AbiParameter {
                    name: "foo".into(),
                    typ: AbiType::Field,
                    visibility: AbiVisibility::Private,
                },
                AbiParameter {
                    name: "signed_example".into(),
                    typ: AbiType::Integer { sign: Sign::Signed, width: 8 },
                    visibility: AbiVisibility::Private,
                },
                AbiParameter {
                    name: "bar".into(),
                    typ: AbiType::Struct {
                        path: "MyStruct".into(),
                        fields: vec![
                            ("field1".into(), AbiType::Integer { sign: Sign::Unsigned, width: 8 }),
                            (
                                "field2".into(),
                                AbiType::Array { length: 2, typ: Box::new(AbiType::Boolean) },
                            ),
                        ],
                    },
                    visibility: AbiVisibility::Private,
                },
            ],
            return_type: Some(AbiReturnType {
                abi_type: AbiType::String { length: 5 },
                visibility: AbiVisibility::Public,
            }),
            error_types: Default::default(),
        };

        let input_map: BTreeMap<String, InputValue> = BTreeMap::from([
            ("foo".into(), InputValue::Field(FieldElement::one())),
            ("signed_example".into(), InputValue::Field(FieldElement::from(240u128))),
            (
                "bar".into(),
                InputValue::Struct(BTreeMap::from([
                    ("field1".into(), InputValue::Field(255u128.into())),
                    (
                        "field2".into(),
                        InputValue::Vec(vec![
                            InputValue::Field(true.into()),
                            InputValue::Field(false.into()),
                        ]),
                    ),
                ])),
            ),
            (MAIN_RETURN_NAME.into(), InputValue::String("hello".to_owned())),
        ]);

        for format in Format::iter() {
            let serialized_inputs = format.serialize(&input_map, &abi).unwrap();

            let reconstructed_input_map = format.parse(&serialized_inputs, &abi).unwrap();

            assert_eq!(input_map, reconstructed_input_map);
        }
    }
}

fn parse_str_to_field(value: &str, arg_name: &str) -> Result<FieldElement, InputParserError> {
    let big_num = if let Some(hex) = value.strip_prefix("0x") {
        BigUint::from_str_radix(hex, 16)
    } else {
        BigUint::from_str_radix(value, 10)
    };
    let bigint = big_num.map_err(|err_msg| InputParserError::ParseStr {
        arg_name: arg_name.into(),
        value: value.into(),
        error: err_msg.to_string(),
    })?;
    if bigint < FieldElement::modulus() {
        Ok(field_from_big_uint(bigint))
    } else {
        Err(InputParserError::InputExceedsFieldModulus {
            arg_name: arg_name.into(),
            value: value.to_string(),
        })
    }
}

fn two_pow(bits: u32) -> BigInt {
    BigInt::one() << bits
}

/// Parses a decimal or `0x`-prefixed hexadecimal integer with an optional leading `-`.
fn parse_str_to_bigint(value: &str, arg_name: &str) -> Result<BigInt, InputParserError> {
    let (sign, magnitude) = match value.strip_prefix('-') {
        Some(magnitude) => (num_bigint::Sign::Minus, magnitude),
        None => (num_bigint::Sign::Plus, value),
    };
    let magnitude = match magnitude.strip_prefix("0x") {
        Some(hex) => BigUint::from_str_radix(hex, 16),
        None => BigUint::from_str_radix(magnitude, 10),
    }
    .map_err(|err| InputParserError::ParseStr {
        arg_name: arg_name.into(),
        value: value.into(),
        error: err.to_string(),
    })?;
    Ok(BigInt::from_biguint(sign, magnitude))
}

/// Converts a scalar of `typ` into the field element that carries it.
///
/// An integer is carried as its fixed-width two's complement bit pattern and is accepted iff it is in the range of its type and that pattern is below the modulus; it is never reduced. A boolean is `0` or `1`. A `Field` is any value below the modulus, a negative one being the negation of its magnitude.
// TODO: take the field as a FieldConfig argument instead of reading the linked modulus, so consumers other than nargo can pass one in.
fn scalar_to_field(
    value: &BigInt,
    typ: &AbiType,
    arg_name: &str,
) -> Result<FieldElement, InputParserError> {
    let (min, max, pattern) = match typ {
        AbiType::Field => {
            if *value.magnitude() >= FieldElement::modulus() {
                return Err(InputParserError::InputExceedsFieldModulus {
                    arg_name: arg_name.into(),
                    value: value.to_string(),
                });
            }
            let field = field_from_big_uint(value.magnitude().clone());
            return Ok(if value.sign() == num_bigint::Sign::Minus { -field } else { field });
        }
        AbiType::Boolean => (BigInt::zero(), BigInt::one(), value.clone()),
        AbiType::Integer { sign: crate::Sign::Unsigned, width } => {
            (BigInt::zero(), two_pow(*width) - 1, value.clone())
        }
        AbiType::Integer { sign: crate::Sign::Signed, width } => {
            let half = two_pow(width - 1);
            let pattern = if value.sign() == num_bigint::Sign::Minus {
                two_pow(*width) + value
            } else {
                value.clone()
            };
            (-half.clone(), half - 1, pattern)
        }
        typ => return Err(InputParserError::AbiTypeMismatch(value.to_string(), typ.clone())),
    };

    if *value < min {
        return Err(InputParserError::InputUnderflowsMinimum {
            arg_name: arg_name.into(),
            value: value.to_string(),
            min: min.to_string(),
        });
    }
    if *value > max {
        return Err(InputParserError::InputOverflowsMaximum {
            arg_name: arg_name.into(),
            value: value.to_string(),
            max: max.to_string(),
        });
    }

    let pattern = pattern.to_biguint().expect("a value in range has a non-negative bit pattern");
    if pattern >= FieldElement::modulus() {
        return Err(InputParserError::InputExceedsFieldModulus {
            arg_name: arg_name.into(),
            value: value.to_string(),
        });
    }
    Ok(field_from_big_uint(pattern))
}

fn field_from_big_uint(bigint: BigUint) -> FieldElement {
    FieldElement::from_be_bytes_reduce(&bigint.to_bytes_be())
}

fn field_to_signed_hex(f: FieldElement, bit_size: u32) -> String {
    let f_u128 = f.to_u128();
    let max = if bit_size == 128 { i128::MAX as u128 } else { (1 << (bit_size - 1)) - 1 };
    if f_u128 > max {
        let f = FieldElement::from(2u32).pow(&bit_size.into()) - f;
        format!("-0x{}", f.to_hex())
    } else {
        format!("0x{}", f.to_hex())
    }
}

#[cfg(test)]
mod tests {
    use acvm::{AcirField, FieldElement};
    use num_bigint::{BigInt, BigUint};
    use strum::IntoEnumIterator;

    use crate::{Abi, AbiParameter, AbiType, AbiVisibility, Sign, errors::InputParserError};

    use super::{Format, InputValue, parse_str_to_bigint, parse_str_to_field, scalar_to_field};

    fn big_uint_from_field(field: FieldElement) -> BigUint {
        BigUint::from_bytes_be(&field.to_be_bytes())
    }

    #[test]
    fn parse_empty_str_fails() {
        // Check that this fails appropriately rather than being treated as 0, etc.
        assert!(parse_str_to_field("", "arg_name").is_err());
    }

    #[test]
    fn parse_fields_from_strings() {
        let fields = vec![
            FieldElement::zero(),
            FieldElement::one(),
            FieldElement::from(u128::MAX) + FieldElement::one(),
            // Equivalent to `FieldElement::modulus() - 1`
            -FieldElement::one(),
        ];

        for field in fields {
            let hex_field = format!("0x{}", field.to_hex());
            let field_from_hex = parse_str_to_field(&hex_field, "arg_name").unwrap();
            assert_eq!(field_from_hex, field);

            let dec_field = big_uint_from_field(field).to_string();
            let field_from_dec = parse_str_to_field(&dec_field, "arg_name").unwrap();
            assert_eq!(field_from_dec, field);
        }
    }

    #[test]
    fn rejects_noncanonical_fields() {
        let noncanonical_field = FieldElement::modulus().to_string();
        assert!(parse_str_to_field(&noncanonical_field, "arg_name").is_err());
    }

    #[test]
    fn quoted_signed_values_are_carried_in_twos_complement() {
        let parse = |value: &str, width| -> Result<FieldElement, InputParserError> {
            let value = parse_str_to_bigint(value, "arg_name")?;
            scalar_to_field(&value, &signed(width), "arg_name")
        };
        assert_eq!(parse("1", 8).unwrap(), FieldElement::from(1_u128));
        assert_eq!(parse("-1", 8).unwrap(), FieldElement::from(255_u128));
        assert_eq!(parse("-1", 16).unwrap(), FieldElement::from(65535_u128));
        assert_eq!(parse("-0x10", 8).unwrap(), FieldElement::from(240_u128));

        assert_eq!(parse("127", 8).unwrap(), FieldElement::from(127_i128));
        assert!(parse("128", 8).is_err());
        assert_eq!(parse("-128", 8).unwrap(), FieldElement::from(128_i128));
        assert!(parse("-129", 8).is_err());

        assert_eq!(parse("32767", 16).unwrap(), FieldElement::from(32767_i128));
        assert!(parse("32768", 16).is_err());
        assert_eq!(parse("-32768", 16).unwrap(), FieldElement::from(32768_i128));
        assert!(parse("-32769", 16).is_err());
    }

    #[test]
    fn test_from_ext() {
        for fmt in Format::iter() {
            assert_eq!(Format::from_ext(fmt.ext()), Some(fmt));
        }
        assert_eq!(Format::from_ext("invalid extension"), None);
    }

    fn abi_with_single_param(typ: AbiType) -> Abi {
        Abi {
            parameters: vec![AbiParameter {
                name: "x".into(),
                typ,
                visibility: AbiVisibility::Private,
            }],
            return_type: None,
            error_types: Default::default(),
        }
    }

    fn parse_scalar(
        format: &Format,
        typ: AbiType,
        spelling: &str,
    ) -> Result<FieldElement, InputParserError> {
        let source = match format {
            Format::Toml => format!("x = {spelling}"),
            Format::Json => format!("{{\"x\": {spelling}}}"),
        };
        let mut inputs = format.parse(&source, &abi_with_single_param(typ))?;
        match inputs.remove("x") {
            Some(InputValue::Field(value)) => Ok(value),
            other => panic!("expected a scalar, got {other:?}"),
        }
    }

    fn unsigned(width: u32) -> AbiType {
        AbiType::Integer { sign: Sign::Unsigned, width }
    }

    fn signed(width: u32) -> AbiType {
        AbiType::Integer { sign: Sign::Signed, width }
    }

    #[test]
    fn unsigned_values_must_fit_their_width_in_every_spelling() {
        for format in Format::iter() {
            for spelling in ["255", "\"255\"", "\"0xff\""] {
                let value = parse_scalar(&format, unsigned(8), spelling).unwrap();
                assert_eq!(value, FieldElement::from(255_u32), "{format:?}: {spelling} as u8");
            }
            for spelling in ["256", "\"256\"", "\"0x100\"", "-1"] {
                assert!(
                    parse_scalar(&format, unsigned(8), spelling).is_err(),
                    "{format:?}: {spelling} is not a u8"
                );
            }
            // A native spelling reaches only i64 magnitudes, whatever the width.
            assert_eq!(
                parse_scalar(&format, unsigned(128), "5").unwrap(),
                FieldElement::from(5_u32)
            );
            assert!(parse_scalar(&format, unsigned(128), "-1").is_err());
        }
    }

    /// The 64-bit values straddle Goldilocks' `p = 2^64 - 2^32 + 1`: `p - 1` is accepted, `p` and `u64::MAX` are rejected; under bn254 all of them are accepted. Only the quoted spelling reaches these magnitudes.
    #[test]
    fn unsigned_values_are_accepted_iff_below_the_modulus() {
        let modulus = FieldElement::modulus();
        for format in Format::iter() {
            for value in
                [u64::MAX - u64::from(u32::MAX), u64::MAX - u64::from(u32::MAX) + 1, u64::MAX]
            {
                let expected =
                    (BigUint::from(value) < modulus).then(|| FieldElement::from(u128::from(value)));
                assert_eq!(
                    parse_scalar(&format, unsigned(64), &format!("\"{value}\"")).ok(),
                    expected,
                    "{format:?}: {value} as u64"
                );
            }
        }
    }

    #[test]
    fn boolean_values_must_be_zero_or_one_in_every_spelling() {
        for format in Format::iter() {
            assert_eq!(
                parse_scalar(&format, AbiType::Boolean, "true").unwrap(),
                FieldElement::one()
            );
            assert_eq!(parse_scalar(&format, AbiType::Boolean, "1").unwrap(), FieldElement::one());
            assert_eq!(
                parse_scalar(&format, AbiType::Boolean, "\"0\"").unwrap(),
                FieldElement::zero()
            );
            for spelling in ["2", "\"2\"", "-1"] {
                assert!(
                    parse_scalar(&format, AbiType::Boolean, spelling).is_err(),
                    "{format:?}: {spelling} is not a bool"
                );
            }
        }
    }

    /// The 64-bit patterns straddle Goldilocks' `p = 2^64 - 2^32 + 1`: `-2^32` is `p - 1` and is accepted, `-2^32 + 1` is `p` and is rejected, and so is `-1`; under bn254 all of them are accepted.
    #[test]
    fn signed_values_are_accepted_iff_their_bit_pattern_is_below_the_modulus() {
        let modulus = FieldElement::modulus();
        let two_64 = BigUint::from(1_u8) << 64;
        let two_32 = BigUint::from(1_u8) << 32;
        let cases = [
            (BigInt::from(1), BigUint::from(1_u8)),
            (BigInt::from(i64::MAX), BigUint::from(i64::MAX as u64)),
            (BigInt::from(i64::MIN), BigUint::from(1_u8) << 63),
            (-BigInt::from(1_u64 << 32), &two_64 - &two_32),
            (-BigInt::from(1_u64 << 32) + 1, &two_64 - &two_32 + 1_u8),
            (BigInt::from(-1), &two_64 - 1_u8),
        ];
        for format in Format::iter() {
            for (value, pattern) in &cases {
                let expected = (*pattern < modulus)
                    .then(|| FieldElement::from_be_bytes_reduce(&pattern.to_bytes_be()));
                for spelling in [value.to_string(), format!("\"{value}\"")] {
                    let actual = parse_scalar(&format, signed(64), &spelling).ok();
                    assert_eq!(actual, expected, "{format:?}: {spelling} as i64");
                }
            }
        }
    }

    #[test]
    fn negative_field_values_are_negated_natively_and_rejected_quoted() {
        for format in Format::iter() {
            assert_eq!(parse_scalar(&format, AbiType::Field, "-1").unwrap(), -FieldElement::one());
            assert!(parse_scalar(&format, AbiType::Field, "\"-1\"").is_err());
        }
    }
}

#[cfg(test)]
mod arbitrary {
    use acvm::{AcirField, FieldElement};
    use proptest::prelude::*;

    use crate::{AbiType, Sign};

    pub(super) fn arb_signed_integer_type_and_value() -> BoxedStrategy<(AbiType, i64)> {
        // At the field's own width a negative value can have a bit pattern at or above the modulus, which the round-trip properties expect to parse.
        (2u32..=64.min(FieldElement::max_num_bits() - 1))
            .prop_flat_map(|width| {
                let typ = Just(AbiType::Integer { width, sign: Sign::Signed });
                let value = if width == 64 {
                    // Avoid overflow
                    i64::MIN..i64::MAX
                } else {
                    -(2i64.pow(width - 1))..(2i64.pow(width - 1) - 1)
                };
                (typ, value)
            })
            .boxed()
    }
}
