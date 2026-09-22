use num_bigint::{BigInt, BigUint};
use num_traits::{Num, One, Zero};
use std::collections::{BTreeMap, HashSet};
use thiserror::Error;

use acvm::{AcirField, FieldConfig, FieldElement};
use itertools::Itertools;
use serde::Serialize;

use crate::errors::InputParserError;
use crate::scalar::{container, not_carried, pattern_of, signed_value};
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
    /// Parses the inputs of `abi` from `input_string`, reading every value in `field`: each
    /// scalar must be the value of its type whose element is below `field`'s modulus.
    pub fn parse(
        &self,
        input_string: &str,
        abi: &Abi,
        field: FieldConfig,
    ) -> Result<BTreeMap<String, InputValue>, InputParserError> {
        match self {
            Format::Json => json::parse_json(input_string, abi, field),
            Format::Toml => toml::parse_toml(input_string, abi, field),
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

    use acvm::{AcirField, FieldConfig, FieldElement};
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

            let reconstructed_input_map =
                format.parse(&serialized_inputs, &abi, FieldConfig::linked()).unwrap();

            assert_eq!(input_map, reconstructed_input_map);
        }
    }
}

/// Refuse a field whose elements the linked element cannot hold, before any value is read.
fn ensure_parsable(field: FieldConfig) -> Result<(), InputParserError> {
    match not_carried(field) {
        Some((field, linked)) => Err(InputParserError::FieldNotCarried { field, linked }),
        None => Ok(()),
    }
}

fn parse_str_to_field(
    value: &str,
    arg_name: &str,
    field: FieldConfig,
) -> Result<FieldElement, InputParserError> {
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
    if bigint < *field.modulus() {
        Ok(container(&bigint))
    } else {
        Err(InputParserError::InputExceedsFieldModulus {
            arg_name: arg_name.into(),
            value: value.to_string(),
            field: field.id(),
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

/// Converts a scalar of `typ` into the element of `field` that carries it.
///
/// An integer is carried as its fixed-width two's complement bit pattern and is accepted iff it is in the range of its type and that pattern is below the modulus; it is never reduced. A boolean is `0` or `1`. A `Field` is any value below the modulus, a negative one being the negation of its magnitude in `field`.
fn scalar_to_field(
    value: &BigInt,
    typ: &AbiType,
    arg_name: &str,
    field: FieldConfig,
) -> Result<FieldElement, InputParserError> {
    let (min, max, pattern) = match typ {
        AbiType::Field => {
            let magnitude = value.magnitude();
            if magnitude >= field.modulus() {
                return Err(InputParserError::InputExceedsFieldModulus {
                    arg_name: arg_name.into(),
                    value: value.to_string(),
                    field: field.id(),
                });
            }
            let element = if value.sign() == num_bigint::Sign::Minus {
                field.modulus() - magnitude
            } else {
                magnitude.clone()
            };
            return Ok(container(&element));
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
    if pattern >= *field.modulus() {
        return Err(InputParserError::InputExceedsFieldModulus {
            arg_name: arg_name.into(),
            value: value.to_string(),
            field: field.id(),
        });
    }
    Ok(container(&pattern))
}

/// The value a signed integer's element spells, in hexadecimal padded to the width of a
/// serialized element, [`FieldConfig::num_bytes`].
fn field_to_signed_hex(f: FieldElement, bit_size: u32) -> String {
    let value = signed_value(&pattern_of(f), bit_size);
    let width = FieldConfig::linked().num_bytes() as usize * 2;
    let sign = if value.sign() == num_bigint::Sign::Minus { "-" } else { "" };
    format!("{sign}0x{:0width$x}", value.magnitude())
}

#[cfg(test)]
mod tests {
    use acvm::{FieldConfig, FieldElement};
    use num_bigint::{BigInt, BigUint};
    use strum::IntoEnumIterator;

    use crate::scalar::{carried_fields, container, uncarried_fields};
    use crate::{Abi, AbiParameter, AbiType, AbiVisibility, Sign, errors::InputParserError};

    use super::{Format, InputValue, parse_str_to_bigint, parse_str_to_field, scalar_to_field};

    #[test]
    fn parse_empty_str_fails() {
        // Check that this fails appropriately rather than being treated as 0, etc.
        for field in carried_fields() {
            assert!(parse_str_to_field("", "arg_name", field).is_err());
        }
    }

    #[test]
    fn parse_fields_from_strings() {
        for field in carried_fields() {
            let largest = field.modulus() - 1u8;
            let values = [BigUint::ZERO, BigUint::from(1u8), BigUint::from(u64::MAX >> 1), largest];

            for value in values {
                let expected = container(&value);
                let hex = format!("0x{value:x}");
                assert_eq!(parse_str_to_field(&hex, "arg_name", field).unwrap(), expected);
                let decimal = value.to_string();
                assert_eq!(parse_str_to_field(&decimal, "arg_name", field).unwrap(), expected);
            }
        }
    }

    #[test]
    fn rejects_noncanonical_fields() {
        for field in carried_fields() {
            for value in [field.modulus().clone(), field.modulus() + 1u8] {
                assert!(parse_str_to_field(&value.to_string(), "arg_name", field).is_err());
            }
        }
    }

    #[test]
    fn quoted_signed_values_are_carried_in_twos_complement() {
        for field in carried_fields() {
            let parse = |value: &str, width| -> Result<FieldElement, InputParserError> {
                let value = parse_str_to_bigint(value, "arg_name")?;
                scalar_to_field(&value, &signed(width), "arg_name", field)
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
        field: FieldConfig,
    ) -> Result<FieldElement, InputParserError> {
        let source = match format {
            Format::Toml => format!("x = {spelling}"),
            Format::Json => format!("{{\"x\": {spelling}}}"),
        };
        let mut inputs = format.parse(&source, &abi_with_single_param(typ), field)?;
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

    /// `p - 1` is the largest element in every spelling that reaches it; `p` and `p + 1` are refused with an error that names the selected field and its modulus.
    #[test]
    fn field_values_are_accepted_iff_below_the_selected_modulus() {
        for field in carried_fields() {
            let modulus = field.modulus();
            let largest = modulus - 1u8;
            for format in Format::iter() {
                for spelling in [format!("\"{largest}\""), format!("\"0x{largest:x}\"")] {
                    assert_eq!(
                        parse_scalar(&format, AbiType::Field, &spelling, field).unwrap(),
                        container(&largest),
                        "{}, {format:?}: {spelling}",
                        field.name()
                    );
                }
                for value in [modulus.clone(), modulus + 1u8] {
                    let error =
                        parse_scalar(&format, AbiType::Field, &format!("\"{value}\""), field)
                            .unwrap_err();
                    let message = error.to_string();
                    assert!(
                        message.contains(&format!("exceeds the {} field modulus", field.name()))
                            && message.contains(&format!("[0, {modulus})")),
                        "{}, {format:?}: {message}",
                        field.name()
                    );
                }
            }
        }
    }

    /// Parsed values are carried in the linked field element, so a field with a larger modulus is refused before any value is read.
    #[test]
    fn a_field_the_linked_element_cannot_hold_is_refused() {
        let linked = FieldConfig::linked().id();
        for field in uncarried_fields() {
            for format in Format::iter() {
                let error = parse_scalar(&format, AbiType::Field, "1", field).unwrap_err();
                assert!(
                    matches!(error, InputParserError::FieldNotCarried { field: refused, linked: held } if refused == field.id() && held == linked),
                    "{}, {format:?}: {error}",
                    field.name()
                );
            }
        }
    }

    #[test]
    fn nested_fields_use_the_selected_modulus() {
        use acvm::FieldId;

        let field = FieldConfig::new(FieldId::Goldilocks);
        if field.modulus() > FieldConfig::linked().modulus() {
            return;
        }
        let abi = abi_with_single_param(AbiType::Struct {
            path: "S".into(),
            fields: vec![(
                "values".into(),
                AbiType::Array { length: 2, typ: Box::new(AbiType::Field) },
            )],
        });
        let largest = field.modulus() - 1u8;
        for format in Format::iter() {
            let source = match format {
                Format::Toml => "[x]\nvalues = [-1, \"1\"]".to_string(),
                Format::Json => "{\"x\": {\"values\": [-1, \"1\"]}}".to_string(),
            };
            let values = format.parse(&source, &abi, field).unwrap();
            let InputValue::Struct(fields) = &values["x"] else { panic!("{format:?}") };
            assert_eq!(
                fields["values"],
                InputValue::Vec(vec![
                    InputValue::Field(container(&largest)),
                    InputValue::Field(1u8.into()),
                ]),
                "{format:?}"
            );
            let serialized = format.serialize(&values, &abi).unwrap();
            assert_eq!(format.parse(&serialized, &abi, field).unwrap(), values);
        }
    }

    /// The widest integer an entry point takes is one bit short of the modulus; every value of it survives a serialize-and-parse round trip, including signed values whose magnitude does not fit 128 bits.
    #[test]
    fn the_widest_integers_round_trip_through_every_format() {
        for field in carried_fields() {
            let width = field.num_bits() - 1;
            let two_to_the_width = BigInt::from(1u8) << width;
            let cases = [
                (unsigned(width), BigInt::ZERO),
                (unsigned(width), &two_to_the_width - 1u8),
                (signed(width), BigInt::ZERO),
                (signed(width), BigInt::from(-1)),
                (signed(width), -(&two_to_the_width >> 1u8)),
                (signed(width), (&two_to_the_width >> 1u8) - 1u8),
                (signed(200.min(width)), -(BigInt::from(1u8) << (200.min(width) - 1))),
            ];
            for (typ, value) in cases {
                for format in Format::iter() {
                    let abi = abi_with_single_param(typ.clone());
                    let parsed = parse_scalar(&format, typ.clone(), &format!("\"{value}\""), field)
                        .unwrap_or_else(|error| {
                            panic!("{}, {format:?}: {value} as {typ:?}: {error}", field.name())
                        });
                    let inputs = [("x".to_string(), InputValue::Field(parsed))].into();
                    let serialized = format.serialize(&inputs, &abi).unwrap();
                    let reparsed = format.parse(&serialized, &abi, field).unwrap();
                    assert_eq!(
                        reparsed,
                        inputs,
                        "{}, {format:?}: {value} as {typ:?}",
                        field.name()
                    );
                }
            }
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
