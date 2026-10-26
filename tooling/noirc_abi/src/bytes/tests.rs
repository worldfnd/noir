//! The golden, accepted and refused inputs are the byte vectors of [`crate::conformance`]; these
//! tests pin the ABI checks, the size cap, each error and its path, and the round-trip
//! properties.

use acvm::acir::circuit::ErrorSelector;
use num_bigint::{BigInt, BigUint};
use num_traits::{One, Zero};
use proptest::prelude::*;
use strum::IntoEnumIterator;

use crate::{
    Abi, AbiErrorType, AbiParameter, AbiReturnType, AbiType, AbiVisibility, Sign,
    input_parser::Format, scalar::signed_value,
};

use super::{
    Error, Inputs, MAX_INTEGER_WIDTH, Value, byte_size, check_abi, decode_params, decode_return,
    encode_params, encode_return, layout, parse, serialize,
};

fn unsigned(width: u32) -> AbiType {
    AbiType::Integer { sign: Sign::Unsigned, width }
}

fn signed(width: u32) -> AbiType {
    AbiType::Integer { sign: Sign::Signed, width }
}

fn array(length: u32, typ: AbiType) -> AbiType {
    AbiType::Array { length, typ: Box::new(typ) }
}

fn tuple(fields: Vec<AbiType>) -> AbiType {
    AbiType::Tuple { fields }
}

fn structure(fields: &[(&str, AbiType)]) -> AbiType {
    let fields = fields.iter().map(|(name, typ)| ((*name).to_string(), typ.clone())).collect();
    AbiType::Struct { path: "S".into(), fields }
}

fn abi(parameters: &[(&str, AbiType)], return_type: Option<AbiType>) -> Abi {
    Abi {
        parameters: parameters
            .iter()
            .map(|(name, typ)| AbiParameter {
                name: (*name).to_string(),
                typ: typ.clone(),
                visibility: AbiVisibility::Private,
            })
            .collect(),
        return_type: return_type
            .map(|abi_type| AbiReturnType { abi_type, visibility: AbiVisibility::Public }),
        error_types: Default::default(),
    }
}

fn single(typ: AbiType) -> Abi {
    abi(&[("x", typ)], None)
}

fn int(value: i64) -> Value {
    Value::Int(value.into())
}

#[test]
fn a_field_is_refused_at_any_depth_of_the_parameters_and_the_return() {
    let cases = [
        (single(array(3, AbiType::Field)), "x[_]"),
        (single(tuple(vec![unsigned(8), AbiType::Field])), "x.1"),
        (single(structure(&[("b", unsigned(8)), ("a", AbiType::Field)])), "x.a"),
        (single(array(2, structure(&[("a", tuple(vec![AbiType::Field]))]))), "x[_].a.0"),
        (abi(&[("x", unsigned(8))], Some(AbiType::Field)), "return"),
        (abi(&[("x", unsigned(8))], Some(array(1, AbiType::Field))), "return[_]"),
    ];
    for (abi, path) in cases {
        let refusal = Error::UnsupportedType { path: path.into(), typ: AbiType::Field };
        assert_eq!(check_abi(&abi), Err(refusal), "{path}");
    }
    assert_eq!(
        byte_size(&structure(&[("a", AbiType::Field)])),
        Err(Error::UnsupportedType { path: "a".into(), typ: AbiType::Field })
    );
}

#[test]
fn error_payloads_may_carry_a_field() {
    let mut abi = single(unsigned(8));
    abi.error_types.insert(ErrorSelector::new(1), AbiErrorType::Custom(AbiType::Field));
    let item_types = vec![AbiType::Field];
    abi.error_types
        .insert(ErrorSelector::new(2), AbiErrorType::FmtString { length: 1, item_types });
    assert_eq!(check_abi(&abi), Ok(()));
}

#[test]
fn integer_widths_run_from_2_to_the_widest() {
    for width in [0, 1, MAX_INTEGER_WIDTH + 1] {
        for typ in [unsigned(width), signed(width)] {
            let refusal = Error::UnsupportedType { path: "x".into(), typ: typ.clone() };
            assert_eq!(check_abi(&single(typ)), Err(refusal));
        }
    }
    for (width, size) in [(2, 1), (8, 1), (9, 2), (MAX_INTEGER_WIDTH, 2048)] {
        for typ in [unsigned(width), signed(width)] {
            assert_eq!(check_abi(&single(typ.clone())), Ok(()));
            assert_eq!(byte_size(&typ), Ok(size));
        }
    }
}

/// Every legal width, including each byte boundary, has the same mathematical extremes.
/// Expected blobs are assembled from bit positions rather than the codec's signed conversion.
#[test]
fn every_integer_width_preserves_its_boundary_patterns() {
    for width in 2..=MAX_INTEGER_WIDTH {
        let length = width.div_ceil(8) as usize;
        let top_bits = (width - 1) % 8 + 1;
        let mut unsigned_max = vec![0xff; length];
        unsigned_max[length - 1] = (0xffu16 >> (8 - top_bits)) as u8;
        let mut signed_min = vec![0; length];
        signed_min[length - 1] = 1 << ((width - 1) % 8);
        let mut signed_max = unsigned_max.clone();
        signed_max[length - 1] ^= signed_min[length - 1];
        let half = BigInt::one() << (width - 1);
        for (typ, min, max, min_bytes, max_bytes) in [
            (
                unsigned(width),
                BigInt::zero(),
                (BigInt::one() << width) - 1,
                vec![0; length],
                unsigned_max,
            ),
            (signed(width), -half.clone(), half - 1, signed_min, signed_max),
        ] {
            let abi = abi(&[("x", typ.clone())], Some(typ));
            for (value, expected) in [(&min, min_bytes), (&max, max_bytes)] {
                let value = Value::Int(value.clone());
                assert_eq!(encode_params(&abi, std::slice::from_ref(&value)), Ok(expected.clone()));
                assert_eq!(decode_params(&abi, &expected), Ok(vec![value.clone()]));
                assert_eq!(encode_return(&abi, &value), Ok(expected.clone()));
                assert_eq!(decode_return(&abi, &expected), Ok(value));
            }
            for value in [&min - 1, &max + 1] {
                assert!(matches!(
                    encode_params(&abi, &[Value::Int(value)]),
                    Err(Error::OutOfRange { .. })
                ));
            }
        }
    }
}

#[test]
fn a_parameterless_unit_entry_has_an_empty_blob_and_no_return_blob() {
    let abi = abi(&[], None);
    assert_eq!(layout(&abi).unwrap().params_len, 0);
    assert_eq!(encode_params(&abi, &[]), Ok(vec![]));
    assert_eq!(decode_params(&abi, &[]), Ok(vec![]));
    assert!(matches!(decode_params(&abi, &[0]), Err(Error::ByteLength { .. })));
    for (format, document) in [(Format::Toml, ""), (Format::Json, "{}")] {
        let inputs = parse(&format, document, &abi).unwrap();
        assert_eq!(inputs, Inputs::default());
        let text = serialize(&format, &abi, &inputs).unwrap();
        assert_eq!(parse(&format, &text, &abi), Ok(inputs));
    }
}

#[test]
fn every_non_boolean_byte_is_refused_in_a_nested_return() {
    let abi = abi(&[], Some(array(2, structure(&[("flag", AbiType::Boolean)]))));
    for byte in 2..=u8::MAX {
        assert_eq!(
            decode_return(&abi, &[1, byte]),
            Err(Error::BadBool { path: "return[1].flag".into(), byte })
        );
    }
}

#[test]
fn utf8_string_lengths_count_bytes_and_raw_strings_keep_every_byte() {
    let typ = structure(&[("text", AbiType::String { length: 2 })]);
    let schema = abi(&[("x", typ.clone())], Some(typ));
    let text = Value::Seq(vec![Value::Str("é".as_bytes().to_vec())]);
    let inputs = Inputs { params: vec![text.clone()], ret: Some(text) };
    for format in Format::iter() {
        let document = serialize(&format, &schema, &inputs).unwrap();
        assert_eq!(parse(&format, &document, &schema), Ok(inputs.clone()));
    }

    let abi = abi(&[("x", AbiType::String { length: 256 })], Some(AbiType::String { length: 256 }));
    let bytes: Vec<u8> = (0..=u8::MAX).collect();
    let value = Value::Str(bytes.clone());
    assert_eq!(encode_params(&abi, std::slice::from_ref(&value)), Ok(bytes.clone()));
    assert_eq!(decode_params(&abi, &bytes), Ok(vec![value.clone()]));
    assert_eq!(encode_return(&abi, &value), Ok(bytes.clone()));
    assert_eq!(decode_return(&abi, &bytes), Ok(value.clone()));
    let inputs = Inputs { params: vec![value], ret: None };
    for format in Format::iter() {
        assert_eq!(serialize(&format, &abi, &inputs), Err(Error::NotUtf8 { path: "x".into() }));
    }
}

/// No entry point takes or returns an empty value, so a type of no bytes is refused wherever it
/// sits. Otherwise an empty element would let an empty blob declare any number of values.
#[test]
fn empty_values_are_refused_at_any_depth() {
    for typ in [array(0, unsigned(8)), AbiType::String { length: 0 }, tuple(vec![]), structure(&[])]
    {
        let refusal =
            |path: &str| Err(Error::UnsupportedType { path: path.into(), typ: typ.clone() });
        assert_eq!(check_abi(&single(typ.clone())), refusal("x"));
        assert_eq!(check_abi(&single(tuple(vec![unsigned(8), typ.clone()]))), refusal("x.1"));
        assert_eq!(check_abi(&abi(&[("x", unsigned(8))], Some(typ.clone()))), refusal("return"));
    }
    let empty = array(0, unsigned(8));
    let huge = single(array(u32::MAX, empty.clone()));
    let refusal = Error::UnsupportedType { path: "x[_]".into(), typ: empty };
    assert_eq!(decode_params(&huge, &[]), Err(refusal));
}

#[test]
fn names_are_unique_and_no_parameter_is_named_return() {
    let duplicate = |path: &str| Err(Error::DuplicateName { path: path.into() });
    let parameters = abi(&[("x", unsigned(8)), ("y", unsigned(8)), ("x", AbiType::Boolean)], None);
    assert_eq!(check_abi(&parameters), duplicate("x"));
    let fields = structure(&[("a", unsigned(8)), ("b", unsigned(8)), ("a", unsigned(8))]);
    assert_eq!(check_abi(&single(fields.clone())), duplicate("x.a"));
    assert_eq!(check_abi(&single(array(2, fields))), duplicate("x[_].a"));

    let reserved = abi(&[("x", unsigned(8)), ("return", unsigned(8))], None);
    assert_eq!(check_abi(&reserved), Err(Error::ReservedName { path: "return".into() }));
    assert_eq!(check_abi(&single(structure(&[("return", unsigned(8))]))), Ok(()));
}

#[test]
fn each_blob_stays_below_2_to_the_32_bytes() {
    let too_large = |path: &str| Error::TooLarge { path: path.into() };
    let widest = array(u32::MAX, unsigned(8));
    assert_eq!(byte_size(&widest), Ok(u32::MAX as usize));
    assert_eq!(byte_size(&array(u32::MAX, unsigned(9))), Err(too_large("")));
    assert_eq!(
        byte_size(&array(2, array(u32::MAX, unsigned(MAX_INTEGER_WIDTH)))),
        Err(too_large("[_]"))
    );
    let half = array(1 << 31, unsigned(8));
    assert_eq!(byte_size(&tuple(vec![half.clone(), half.clone()])), Err(too_large("")));
    assert_eq!(
        byte_size(&structure(&[("a", half.clone()), ("b", half.clone())])),
        Err(too_large(""))
    );

    // The parameters count together, the return value on its own.
    let one_short = array((1 << 31) - 1, unsigned(8));
    let fits = abi(&[("x", half.clone()), ("y", one_short)], Some(widest));
    let layout = layout(&fits).unwrap();
    assert_eq!(layout.params_len, u32::MAX as usize);
    assert_eq!(layout.params, [0..1 << 31, 1 << 31..u32::MAX as usize]);
    assert_eq!(layout.ret_len, Some(u32::MAX as usize));
    assert_eq!(check_abi(&abi(&[("x", half.clone()), ("y", half)], None)), Err(too_large("y")));
    let ret = Some(array(1 << 31, unsigned(16)));
    assert_eq!(check_abi(&abi(&[("x", unsigned(8))], ret)), Err(too_large("return")));
}

#[test]
fn a_short_blob_for_a_huge_layout_is_refused_before_anything_is_allocated() {
    let widest = array(u32::MAX, unsigned(8));
    let abi = abi(&[("x", widest.clone())], Some(widest));
    let expected = u32::MAX as usize;
    assert_eq!(
        decode_params(&abi, &[0]),
        Err(Error::ByteLength { path: String::new(), expected, found: 1 })
    );
    assert_eq!(
        decode_return(&abi, &[]),
        Err(Error::ByteLength { path: "return".into(), expected, found: 0 })
    );
}

#[test]
fn the_return_value_is_its_own_blob() {
    let abi = abi(&[("x", unsigned(8))], Some(tuple(vec![signed(9), AbiType::Boolean])));
    let ret = Value::Seq(vec![int(-2), Value::Bool(true)]);
    for (format, document) in [
        (Format::Toml, "x = 5\nreturn = [-2, true]"),
        (Format::Json, r#"{"return": [-2, true], "x": 5}"#),
    ] {
        let inputs = parse(&format, document, &abi).unwrap();
        assert_eq!(inputs, Inputs { params: vec![int(5)], ret: Some(ret.clone()) });
    }
    assert_eq!(parse(&Format::Json, r#"{"x": 5}"#, &abi).unwrap().ret, None);
    assert_eq!(encode_params(&abi, &[int(5)]), Ok(vec![0x05]));
    assert_eq!(encode_return(&abi, &ret), Ok(vec![0xfe, 0x01, 0x01]));
    assert_eq!(decode_return(&abi, &[0xfe, 0x01, 0x01]), Ok(ret));
    assert_eq!(
        decode_return(&abi, &[0xfe, 0x03, 0x01]),
        Err(Error::Padding { path: "return.0".into() })
    );
    assert_eq!(layout(&abi).unwrap().ret_len, Some(3));

    for blob in [vec![0xfe, 0x01], vec![0xfe, 0x01, 0x01, 0x00]] {
        let found = blob.len();
        let refusal = Error::ByteLength { path: "return".into(), expected: 3, found };
        assert_eq!(decode_return(&abi, &blob), Err(refusal));
    }

    let none = single(unsigned(8));
    let unknown = Error::UnknownKey { path: "return".into() };
    assert_eq!(encode_return(&none, &int(1)), Err(unknown.clone()));
    assert_eq!(decode_return(&none, &[0x01]), Err(unknown.clone()));
    let inputs = Inputs { params: vec![int(1)], ret: Some(int(1)) };
    assert_eq!(serialize(&Format::Toml, &none, &inputs), Err(unknown));
}

/// A parameter `x` holding three structs, the third of them spelled by the case.
fn cells() -> Abi {
    let cell = structure(&[
        ("head", tuple(vec![unsigned(8), AbiType::Boolean])),
        ("tail", AbiType::String { length: 1 }),
    ]);
    abi(&[("x", array(3, cell))], Some(unsigned(8)))
}

#[test]
fn reading_errors_name_the_path_of_the_value() {
    let abi = cells();
    let toml = |third: &str| {
        format!(
            "x = [{{ head = [1, true], tail = \"a\" }}, {{ head = [2, false], tail = \"b\" }}, {third}]"
        )
    };
    let path = |path: &str| path.to_string();
    let cases = [
        (
            "{ head = [256, true], tail = \"c\" }",
            Error::OutOfRange { path: path("x[2].head.0"), value: 256.into(), typ: unsigned(8) },
        ),
        (
            "{ head = [3, 1], tail = \"c\" }",
            Error::Shape { path: path("x[2].head.1"), expected: "a boolean", found: "an integer" },
        ),
        (
            "{ head = [3.0, true], tail = \"c\" }",
            Error::Shape { path: path("x[2].head.0"), expected: "an integer", found: "a float" },
        ),
        (
            "{ head = [3], tail = \"c\" }",
            Error::Length { path: path("x[2].head"), expected: 2, found: 1 },
        ),
        (
            "{ head = [3, true], tail = \"cd\" }",
            Error::Length { path: path("x[2].tail"), expected: 1, found: 2 },
        ),
        ("{ head = [3, true] }", Error::Missing { path: path("x[2].tail") }),
        (
            "{ head = [3, true], tail = \"c\", tip = 1 }",
            Error::UnknownKey { path: path("x[2].tip") },
        ),
        (
            "{ head = [\"+3\", true], tail = \"c\" }",
            Error::Spelling {
                path: path("x[2].head.0"),
                text: "+3".into(),
                rule: "a quoted unsigned integer is spelled 0x[0-9a-fA-F]+ or [0-9]+",
            },
        ),
        ("[3, true]", Error::Shape { path: path("x[2]"), expected: "a table", found: "an array" }),
    ];
    for (third, error) in cases {
        assert_eq!(parse(&Format::Toml, &toml(third), &abi), Err(error), "{third}");
    }

    let json = |rest: &str| {
        format!(
            r#"{{"x": [{{"head": [1, true], "tail": "a"}}, {{"head": [2, false], "tail": "b"}}, {rest}"#
        )
    };
    let third = r#"{"head": [3, true], "tail": "c"}]"#;
    let cases = [
        (
            json(r#"{"head": [3, true], "head": [3, true], "tail": "c"}]}"#),
            Error::DuplicateKey { path: path("x[2].head") },
        ),
        (json(&format!(r#"{third}, "x": []}}"#)), Error::DuplicateKey { path: path("x") }),
        (json(&format!(r#"{third}, "y": 1}}"#)), Error::UnknownKey { path: path("y") }),
        (
            json(&format!(r#"{third}, "return": null}}"#)),
            Error::Shape { path: path("return"), expected: "an integer", found: "null" },
        ),
        (
            json(&format!(r#"{third}, "return": 0.0}}"#)),
            Error::Shape { path: path("return"), expected: "an integer", found: "a float" },
        ),
        (
            json(&format!(r#"{third}, "return": 9223372036854775808}}"#)),
            Error::Spelling {
                path: path("return"),
                text: "9223372036854775808".into(),
                rule: "a native integer is at most i64::MAX; quote a larger one",
            },
        ),
        (
            json(r#"{"head": [3, true], "tail": "c"}, 4]}"#),
            Error::Length { path: path("x"), expected: 3, found: 4 },
        ),
        (
            json(r#"{"head": [3, true], "tail": "c"}]}"#).replace(r#""tail": "a""#, r#""tail": 1"#),
            Error::Shape { path: path("x[0].tail"), expected: "a string", found: "an integer" },
        ),
        ("{}".into(), Error::Missing { path: path("x") }),
        (
            "[]".into(),
            Error::Shape { path: String::new(), expected: "an object", found: "an array" },
        ),
    ];
    for (document, error) in cases {
        assert_eq!(parse(&Format::Json, &document, &abi), Err(error), "{document}");
    }

    for (format, document) in [
        (Format::Toml, "x = 1\nx = 2"),
        (Format::Toml, "x = "),
        (Format::Json, r#"{"x": "#),
        (Format::Json, r#"{"x": 1} 2"#),
    ] {
        let parsed = parse(&format, document, &single(unsigned(8)));
        assert!(matches!(parsed, Err(Error::Syntax { .. })), "{document}: {parsed:?}");
    }
    let parsed = parse(&Format::Toml, "x = 1979-05-27", &single(unsigned(8)));
    let datetime = Error::Shape { path: path("x"), expected: "an integer", found: "a datetime" };
    assert_eq!(parsed, Err(datetime));
}

#[test]
fn decoding_and_encoding_errors_name_the_path_of_the_value() {
    let abi = cells();
    let mut blob = vec![0x01, 0x01, b'a', 0x02, 0x00, b'b', 0x03, 0x01, b'c'];
    assert!(decode_params(&abi, &blob).is_ok());
    blob[4] = 0x02;
    assert_eq!(
        decode_params(&abi, &blob),
        Err(Error::BadBool { path: "x[1].head.1".into(), byte: 0x02 })
    );
    let padded = single(tuple(vec![unsigned(8), array(2, signed(9))]));
    assert_eq!(
        decode_params(&padded, &[0x00, 0x00, 0x00, 0xff, 0x02]),
        Err(Error::Padding { path: "x.1[1]".into() })
    );

    let cell = |head: Value, tail: &[u8]| {
        Value::Seq(vec![Value::Seq(vec![head, Value::Bool(true)]), Value::Str(tail.to_vec())])
    };
    let cells =
        |third: Value| vec![Value::Seq(vec![cell(int(1), b"a"), cell(int(2), b"b"), third])];
    let cases = [
        (
            cells(cell(int(3), b"cd")),
            Error::Length { path: "x[2].tail".into(), expected: 1, found: 2 },
        ),
        (
            cells(cell(int(-1), b"c")),
            Error::OutOfRange { path: "x[2].head.0".into(), value: (-1).into(), typ: unsigned(8) },
        ),
        (
            cells(cell(Value::Bool(true), b"c")),
            Error::Shape { path: "x[2].head.0".into(), expected: "an integer", found: "a boolean" },
        ),
        (cells(Value::Seq(vec![])), Error::Length { path: "x[2]".into(), expected: 2, found: 0 }),
        (vec![], Error::Length { path: String::new(), expected: 1, found: 0 }),
    ];
    for (params, error) in cases {
        assert_eq!(encode_params(&abi, &params), Err(error.clone()));
        let inputs = Inputs { params, ret: None };
        assert_eq!(serialize(&Format::Json, &abi, &inputs), Err(error));
    }
}

/// A document with one parameter `x` spelled `spelling`.
fn document(format: &Format, spelling: &str) -> String {
    match format {
        Format::Toml => format!("x = {spelling}"),
        Format::Json => format!("{{\"x\": {spelling}}}"),
    }
}

/// A quoted integer outside `-?(0x[0-9a-fA-F]+|[0-9]+)` is a spelling error, whatever its value.
#[test]
fn a_quoted_integer_outside_the_grammar_is_a_spelling_error() {
    for (typ, text) in [
        (unsigned(8), ""),
        (unsigned(8), " 5"),
        (unsigned(8), "5 "),
        (unsigned(8), "\u{665}"),
        (unsigned(16), "1_000"),
        (unsigned(8), "-0"),
        (signed(8), "+5"),
        (signed(8), "- 5"),
        (signed(8), "0x-1"),
        (signed(8), "0X10"),
    ] {
        for format in Format::iter() {
            let parsed =
                parse(&format, &document(&format, &format!("\"{text}\"")), &single(typ.clone()));
            assert!(
                matches!(&parsed, Err(Error::Spelling { path, text: found, .. }) if path == "x" && found == text),
                "{format:?}, {text:?}: {parsed:?}"
            );
        }
    }
}

/// Leading zeros aside, a quoted integer of more digits than the widest type has bits is refused
/// before it is converted; one within the limit is converted, then checked against its type.
#[test]
fn a_quoted_integer_has_at_most_as_many_digits_as_the_widest_type_has_bits() {
    let digits = MAX_INTEGER_WIDTH as usize;
    let abi = single(unsigned(MAX_INTEGER_WIDTH));
    for format in Format::iter() {
        let read = |text: String| parse(&format, &document(&format, &format!("\"{text}\"")), &abi);
        assert_eq!(
            read(format!("{}5", "0".repeat(2 * digits))).map(|inputs| inputs.params),
            Ok(vec![int(5)])
        );
        let refused = read(format!("1{}", "0".repeat(digits)));
        assert!(
            matches!(&refused, Err(Error::Spelling { rule, .. }) if rule.contains("16384 digits")),
            "{format:?}: {refused:?}"
        );
        let converted = read("9".repeat(digits));
        assert!(matches!(converted, Err(Error::OutOfRange { .. })), "{format:?}: {converted:?}");
    }
}

/// A quoted hex integer spells a magnitude, not a two's complement pattern.
#[test]
fn quoted_hex_spells_the_magnitude_of_a_signed_integer() {
    let abi = single(signed(8));
    for format in Format::iter() {
        let read = |text: &str| parse(&format, &document(&format, &format!("\"{text}\"")), &abi);
        assert_eq!(read("0x7f").map(|inputs| inputs.params), Ok(vec![int(127)]));
        assert_eq!(read("-0x80").map(|inputs| inputs.params), Ok(vec![int(-128)]));
        for (text, value) in [("0x80", 128), ("0xff", 255), ("-0x81", -129)] {
            let refusal =
                Error::OutOfRange { path: "x".into(), value: value.into(), typ: signed(8) };
            assert_eq!(read(text), Err(refusal), "{format:?}, {text}");
        }
    }
}

/// `serialize` writes integers as quoted decimal, parameters and struct fields in declaration
/// order and `return` last; TOML writes a table after the plain values beside it.
#[test]
fn serialize_writes_quoted_decimal_in_declaration_order() {
    let pair = structure(&[("y", AbiType::Boolean), ("b", unsigned(8))]);
    let abi = abi(&[("zed", unsigned(8)), ("s", pair), ("a", signed(9))], Some(unsigned(8)));
    let inputs = Inputs {
        params: vec![int(5), Value::Seq(vec![Value::Bool(true), int(255)]), int(-2)],
        ret: Some(int(7)),
    };
    assert_eq!(
        serialize(&Format::Json, &abi, &inputs).unwrap(),
        r#"{"zed":"5","s":{"y":true,"b":"255"},"a":"-2","return":"7"}"#
    );
    assert_eq!(
        serialize(&Format::Toml, &abi, &inputs).unwrap(),
        "zed = \"5\"\na = \"-2\"\nreturn = \"7\"\n\n[s]\ny = true\nb = \"255\"\n"
    );
}

/// The names of generated struct fields and parameters, none in alphabetical order.
const FIELD_NAMES: [&str; 4] = ["zeta", "alpha", "mid", "beta"];
const PARAMETER_NAMES: [&str; 4] = ["zed", "a", "mid", "b"];

/// Mostly widths below 300 bits, sometimes the widest.
fn arb_width() -> impl Strategy<Value = u32> {
    prop_oneof![8 => 2u32..=72, 3 => 73u32..=300, 1 => Just(MAX_INTEGER_WIDTH)]
}

/// A field-free type with no empty value in it, at most three levels deep, of at most four
/// elements per level.
fn arb_type() -> impl Strategy<Value = AbiType> {
    let leaf = prop_oneof![
        6 => (any::<bool>(), arb_width())
            .prop_map(|(is_signed, width)| if is_signed { signed(width) } else { unsigned(width) }),
        2 => Just(AbiType::Boolean),
        2 => (1u32..=6).prop_map(|length| AbiType::String { length }),
    ];
    leaf.prop_recursive(3, 24, 4, |inner| {
        prop_oneof![
            (1u32..=4, inner.clone()).prop_map(|(length, typ)| array(length, typ)),
            prop::collection::vec(inner.clone(), 1..=3).prop_map(tuple),
            prop::collection::vec(inner, 1..=4).prop_map(|types| {
                let fields = FIELD_NAMES.iter().map(|name| name.to_string()).zip(types).collect();
                AbiType::Struct { path: "S".into(), fields }
            }),
        ]
    })
}

/// A value of the integer type, often one of its extremes.
fn arb_int(sign: Sign, width: u32) -> impl Strategy<Value = BigInt> {
    let random =
        prop::collection::vec(any::<u8>(), width.div_ceil(8) as usize).prop_map(move |bytes| {
            let pattern = BigUint::from_bytes_le(&bytes) % (BigUint::one() << width);
            match sign {
                Sign::Unsigned => BigInt::from(pattern),
                Sign::Signed => signed_value(&pattern, width),
            }
        });
    let extremes = match sign {
        Sign::Unsigned => vec![BigInt::zero(), (BigInt::one() << width) - 1],
        Sign::Signed => {
            let half = BigInt::one() << (width - 1);
            vec![BigInt::zero(), BigInt::from(-1), -half.clone(), half - 1]
        }
    };
    prop_oneof![3 => random, 1 => prop::sample::select(extremes)]
}

/// A value of `typ`; strings are ASCII.
fn arb_value(typ: &AbiType) -> BoxedStrategy<Value> {
    match typ {
        AbiType::Integer { sign, width } => arb_int(*sign, *width).prop_map(Value::Int).boxed(),
        AbiType::Boolean => any::<bool>().prop_map(Value::Bool).boxed(),
        AbiType::String { length } => {
            prop::collection::vec(0u8..=0x7f, *length as usize).prop_map(Value::Str).boxed()
        }
        AbiType::Array { length, typ } => {
            prop::collection::vec(arb_value(typ), *length as usize).prop_map(Value::Seq).boxed()
        }
        AbiType::Tuple { fields } => {
            fields.iter().map(arb_value).collect::<Vec<_>>().prop_map(Value::Seq).boxed()
        }
        AbiType::Struct { fields, .. } => {
            let fields: Vec<_> = fields.iter().map(|(_, typ)| arb_value(typ)).collect();
            fields.prop_map(Value::Seq).boxed()
        }
        AbiType::Field => unreachable!("the strategies make no `Field`"),
    }
}

/// An ABI of up to three parameters and maybe a return value, and inputs for it.
fn arb_case() -> impl Strategy<Value = (Abi, Inputs)> {
    (prop::collection::vec(arb_type(), 0..=3), prop::option::of(arb_type())).prop_flat_map(
        |(types, ret)| {
            let parameters: Vec<_> =
                PARAMETER_NAMES.into_iter().zip(types.iter().cloned()).collect();
            let abi = abi(&parameters, ret.clone());
            let params: Vec<_> = types.iter().map(arb_value).collect();
            let ret = match &ret {
                Some(typ) => arb_value(typ).prop_map(Some).boxed(),
                None => Just(None).boxed(),
            };
            (Just(abi), params, ret).prop_map(|(abi, params, ret)| (abi, Inputs { params, ret }))
        },
    )
}

/// Appends the position of every padding bit in the encoding of a value of `typ`, as a byte and
/// a bit, counting bytes from `offset`.
fn padding_bits(typ: &AbiType, offset: &mut usize, bits: &mut Vec<(usize, u32)>) {
    match typ {
        AbiType::Integer { width, .. } => {
            let length = width.div_ceil(8) as usize;
            if width % 8 != 0 {
                bits.extend((width % 8..8).map(|bit| (*offset + length - 1, bit)));
            }
            *offset += length;
        }
        AbiType::Boolean => *offset += 1,
        AbiType::String { length } => *offset += *length as usize,
        AbiType::Array { length, typ } => {
            for _ in 0..*length {
                padding_bits(typ, offset, bits);
            }
        }
        AbiType::Tuple { fields } => {
            for typ in fields {
                padding_bits(typ, offset, bits);
            }
        }
        AbiType::Struct { fields, .. } => {
            for (_, typ) in fields {
                padding_bits(typ, offset, bits);
            }
        }
        AbiType::Field => unreachable!("the strategies make no `Field`"),
    }
}

proptest! {
    #[test]
    fn decoding_an_encoding_returns_the_value((abi, inputs) in arb_case()) {
        let blob = encode_params(&abi, &inputs.params).unwrap();
        prop_assert_eq!(decode_params(&abi, &blob).unwrap(), inputs.params);
        if let Some(ret) = &inputs.ret {
            let blob = encode_return(&abi, ret).unwrap();
            prop_assert_eq!(&decode_return(&abi, &blob).unwrap(), ret);
        }
    }

    #[test]
    fn parsing_a_serialization_returns_the_inputs((abi, inputs) in arb_case()) {
        for format in Format::iter() {
            let document = serialize(&format, &abi, &inputs).unwrap();
            let parsed = parse(&format, &document, &abi);
            prop_assert_eq!(parsed.as_ref(), Ok(&inputs), "{:?}:\n{}", format, document);
        }
    }

    #[test]
    fn each_layout_range_holds_that_parameters_own_encoding((abi, inputs) in arb_case()) {
        let layout = layout(&abi).unwrap();
        let blob = encode_params(&abi, &inputs.params).unwrap();
        prop_assert_eq!(layout.params_len, blob.len());
        let mut end = 0;
        for ((param, value), range) in abi.parameters.iter().zip(&inputs.params).zip(&layout.params) {
            prop_assert_eq!(range.start, end);
            end = range.end;
            let alone = Abi { parameters: vec![param.clone()], ..Abi::default() };
            let own = encode_params(&alone, std::slice::from_ref(value)).unwrap();
            prop_assert_eq!(&blob[range.clone()], own.as_slice());
            prop_assert_eq!(byte_size(&param.typ), Ok(range.len()));
        }
        prop_assert_eq!(end, layout.params_len);
        let ret_len = inputs.ret.as_ref().map(|ret| encode_return(&abi, ret).unwrap().len());
        prop_assert_eq!(layout.ret_len, ret_len);
    }

    #[test]
    fn flipping_a_padding_bit_is_refused(
        (abi, inputs) in arb_case(),
        params_choice in any::<prop::sample::Index>(),
        ret_choice in any::<prop::sample::Index>(),
    ) {
        let mut blobs = vec![(
            encode_params(&abi, &inputs.params).unwrap(),
            abi.parameters.iter().map(|param| &param.typ).collect::<Vec<_>>(),
            params_choice,
        )];
        if let (Some(ret), Some(typ)) = (&inputs.ret, &abi.return_type) {
            blobs.push((encode_return(&abi, ret).unwrap(), vec![&typ.abi_type], ret_choice));
        }
        for (index, (blob, types, choice)) in blobs.into_iter().enumerate() {
            let (mut offset, mut bits) = (0, Vec::new());
            for typ in types {
                padding_bits(typ, &mut offset, &mut bits);
            }
            prop_assert_eq!(offset, blob.len());
            if bits.is_empty() {
                continue;
            }
            let (byte, bit) = bits[choice.index(bits.len())];
            let mut flipped = blob;
            flipped[byte] ^= 1 << bit;
            let decoded = if index == 0 {
                decode_params(&abi, &flipped).map(drop)
            } else {
                decode_return(&abi, &flipped).map(drop)
            };
            prop_assert!(matches!(decoded, Err(Error::Padding { .. })), "{:?}", decoded);
        }
    }
}
