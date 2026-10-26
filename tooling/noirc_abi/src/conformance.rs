//! Accept and reject vectors shared by the parsers and the ast-interpreter input bridge:
//! [`boundary_vectors`] for the single-element ABI domain, where each scalar is one field
//! element, and [`byte_vectors`] for the byte codec, [`crate::bytes`].

use acvm::FieldConfig;
use num_bigint::{BigInt, BigUint};

use crate::{Abi, AbiParameter, AbiReturnType, AbiType, AbiVisibility, Sign};

/// One spelling of a scalar input and the element it must be carried in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryVector {
    pub typ: AbiType,
    /// The value as written after `x = ` in `Prover.toml`: a TOML integer, a boolean, or a quoted
    /// string. The same text is a valid JSON value.
    pub spelling: String,
    /// The element the value is carried in, or `None` if the parser must refuse it.
    pub element: Option<BigUint>,
}

/// The boundary vectors of `field`: `p - 1`, `p` and `p + 1` for `Field`, the two
/// signednesses at the widest width the parser carries (one bit short of the modulus, which a
/// hand-written ABI can still name) and one bit wider, the 64-bit values that
/// straddle the Goldilocks modulus, a native spelling of a wide type, and the spellings each
/// scalar type refuses whatever the field.
pub fn boundary_vectors(field: FieldConfig) -> Vec<BoundaryVector> {
    let modulus = field.modulus();
    let widest = field.num_bits() - 1;
    let canonical = |element: BigUint| (element < *modulus).then_some(element);
    let mut vectors = Vec::new();
    let mut push = |typ: &AbiType, spelling: String, element: Option<BigUint>| {
        vectors.push(BoundaryVector { typ: typ.clone(), spelling, element });
    };

    let typ = AbiType::Field;
    push(&typ, "0".into(), Some(BigUint::ZERO));
    push(&typ, "1".into(), Some(BigUint::from(1u8)));
    push(&typ, "-1".into(), Some(modulus - 1u8));
    push(&typ, "\"-1\"".into(), None);
    push(&typ, format!("\"{}\"", modulus - 1u8), Some(modulus - 1u8));
    push(&typ, format!("\"0x{:x}\"", modulus - 1u8), Some(modulus - 1u8));
    push(&typ, format!("\"{modulus}\""), None);
    push(&typ, format!("\"{}\"", modulus + 1u8), None);

    let typ = AbiType::Boolean;
    for (spelling, element) in [("true", 1u8), ("false", 0), ("1", 1), ("\"0\"", 0)] {
        push(&typ, spelling.into(), Some(BigUint::from(element)));
    }
    for spelling in ["2", "\"2\"", "-1"] {
        push(&typ, spelling.into(), None);
    }

    let typ = AbiType::Integer { sign: Sign::Unsigned, width: 8 };
    push(&typ, "255".into(), Some(BigUint::from(255u8)));
    push(&typ, "\"0xff\"".into(), Some(BigUint::from(255u8)));
    for spelling in ["256", "\"0x100\"", "-1"] {
        push(&typ, spelling.into(), None);
    }

    // A native spelling reaches only i64 magnitudes, whatever the width.
    let typ = AbiType::Integer { sign: Sign::Unsigned, width: 128 };
    push(&typ, "5".into(), Some(BigUint::from(5u8)));
    push(&typ, "-1".into(), None);

    // The Goldilocks modulus is 2^64 - 2^32 + 1, so these values sit on either side of it.
    let typ = AbiType::Integer { sign: Sign::Unsigned, width: 64 };
    for value in [u64::MAX - u64::from(u32::MAX), u64::MAX - u64::from(u32::MAX) + 1, u64::MAX] {
        push(&typ, format!("\"{value}\""), canonical(BigUint::from(value)));
    }
    let typ = AbiType::Integer { sign: Sign::Signed, width: 64 };
    let two_to_the_32 = BigInt::from(1u64 << 32);
    for value in [
        BigInt::from(1),
        BigInt::from(i64::MAX),
        BigInt::from(i64::MIN),
        -two_to_the_32.clone(),
        -two_to_the_32 + 1,
        BigInt::from(-1),
    ] {
        let element = canonical(pattern(&value, 64));
        push(&typ, value.to_string(), element.clone());
        push(&typ, format!("\"{value}\""), element);
    }

    let two_to_the_widest = BigUint::from(1u8) << widest;
    let typ = AbiType::Integer { sign: Sign::Unsigned, width: widest };
    push(&typ, format!("\"{}\"", &two_to_the_widest - 1u8), Some(&two_to_the_widest - 1u8));
    push(&typ, format!("\"{two_to_the_widest}\""), None);
    let typ = AbiType::Integer { sign: Sign::Signed, width: widest };
    let half = BigInt::from(&two_to_the_widest >> 1u8);
    push(&typ, "-1".into(), Some(&two_to_the_widest - 1u8));
    push(&typ, format!("\"{}\"", -half.clone()), Some(&two_to_the_widest >> 1u8));
    push(&typ, format!("\"{}\"", &half - 1), Some((&two_to_the_widest >> 1u8) - 1u8));
    push(&typ, format!("\"{half}\""), None);
    push(&typ, format!("\"{}\"", -half - 1), None);

    // A width wider than one field element, which nargo's circuit path never compiles, still
    // reaches a hand-written ABI, where the parser keeps the values below the modulus.
    let typ = AbiType::Integer { sign: Sign::Unsigned, width: widest + 1 };
    push(&typ, format!("\"{}\"", modulus - 1u8), Some(modulus - 1u8));
    push(&typ, format!("\"{modulus}\""), None);

    vectors
}

/// The fixed-width two's complement bit pattern of `value`.
fn pattern(value: &BigInt, width: u32) -> BigUint {
    let modulus = BigInt::from(1u8) << width;
    ((value % &modulus + &modulus) % &modulus).to_biguint().expect("a remainder is non-negative")
}

/// One case of the byte codec, [`crate::bytes`]: an input document in each format and a
/// parameter blob, accepted or refused together.
#[derive(Debug, Clone)]
pub struct ByteVector {
    /// What the case pins.
    pub name: String,
    /// The ABI the documents and the blob are read against, which [`crate::bytes::check_abi`]
    /// accepts.
    pub abi: Abi,
    /// The input as a TOML document, or `None` when the case has no TOML form.
    pub toml: Option<String>,
    /// The input as a JSON document, or `None` when the case has no JSON form.
    pub json: Option<String>,
    /// The parameter blob, or `None` when the case has no decode form. In an accepted case it is
    /// the encoding of the documents' parameters, and decodes to them.
    pub bytes: Option<Vec<u8>>,
    /// Whether the documents parse and the blob decodes. In a refused case each is refused.
    pub accept: bool,
}

/// The byte vectors: golden encodings, including integers wider than any field and a nested
/// struct, tuple and array whose parameters and fields are not in alphabetical order; the
/// spellings the strict parser accepts and refuses in each format; and the blobs the decoder
/// refuses.
pub fn byte_vectors() -> Vec<ByteVector> {
    let unsigned = |width| AbiType::Integer { sign: Sign::Unsigned, width };
    let signed = |width| AbiType::Integer { sign: Sign::Signed, width };
    let two_to_the = |bits: u32| -> BigInt { BigInt::from(1u8) << bits };
    let mut vectors = Vec::new();

    let golden = [
        ("i17 -1", signed(17), "-1".to_string(), vec![0xff, 0xff, 0x01]),
        ("i17 min", signed(17), "-65536".into(), vec![0x00, 0x00, 0x01]),
        ("i17 max", signed(17), "65535".into(), vec![0xff, 0xff, 0x00]),
        (
            "i200 min",
            signed(200),
            format!("\"{}\"", -two_to_the(199)),
            [vec![0; 24], vec![0x80]].concat(),
        ),
        (
            "i200 max",
            signed(200),
            format!("\"{}\"", two_to_the(199) - 1),
            [vec![0xff; 24], vec![0x7f]].concat(),
        ),
        ("u16384 max", unsigned(16384), format!("\"0x{}\"", "f".repeat(4096)), vec![0xff; 2048]),
        ("i2 min", signed(2), "-2".into(), vec![0x02]),
        ("u2 max", unsigned(2), "3".into(), vec![0x03]),
        ("bool true", AbiType::Boolean, "true".into(), vec![0x01]),
        ("str<3> abc", AbiType::String { length: 3 }, "\"abc\"".into(), vec![0x61, 0x62, 0x63]),
    ];
    for (name, typ, spelling, bytes) in golden {
        vectors.push(value_vector(name, typ, &spelling, Some(bytes), true));
    }

    // The extremes of each signedness at widths around every byte and field boundary, and one
    // past them.
    for width in [2, 7, 8, 9, 63, 64, 65, 127, 128, 129, 253, 254, 255, 256, 16384] {
        let max: BigInt = two_to_the(width) - 1;
        let half = two_to_the(width - 1);
        for (sign, typ, value, accept) in [
            ("u", unsigned(width), max.clone(), true),
            ("u", unsigned(width), max + 1, false),
            ("i", signed(width), -half.clone(), true),
            ("i", signed(width), &half - 1, true),
            ("i", signed(width), -half.clone() - 1, false),
            ("i", signed(width), half, false),
        ] {
            let bytes = accept.then(|| le_bytes(&value, width));
            let name = format!("{sign}{width} {value}");
            vectors.push(value_vector(&name, typ, &format!("\"{value}\""), bytes, accept));
        }
    }

    // Parameters and struct fields are read and laid out in declaration order.
    let packet = AbiType::Struct {
        path: "Packet".into(),
        fields: vec![
            ("tag".into(), unsigned(8)),
            ("head".into(), AbiType::Tuple { fields: vec![AbiType::Boolean, signed(9)] }),
            ("body".into(), AbiType::Array { length: 2, typ: Box::new(unsigned(16)) }),
            ("name".into(), AbiType::String { length: 2 }),
        ],
    };
    let nested = abi(vec![("zeta", unsigned(8)), ("x", packet)], None);
    let nested_bytes = vec![0x09, 0x07, 0x01, 0xfe, 0x01, 0x02, 0x01, 0xff, 0xff, 0x68, 0x69];
    let nested_toml = |fields: &str| format!("zeta = 9\n\n[x]\n{fields}");
    let nested_json = |fields: &str| format!("{{\"zeta\": 9, \"x\": {{{fields}}}}}");
    let toml_fields = "tag = 7\nhead = [true, -2]\nbody = [258, \"0xffff\"]\nname = \"hi\"\n";
    let json_fields =
        "\"tag\": 7, \"head\": [true, -2], \"body\": [258, \"0xffff\"], \"name\": \"hi\"";
    vectors.push(document(
        "nested golden",
        &nested,
        Some(nested_toml(toml_fields)),
        Some(nested_json(json_fields)),
        Some(nested_bytes.clone()),
        true,
    ));
    vectors.push(document(
        "nested unknown key",
        &nested,
        Some(nested_toml(&format!("{toml_fields}extra = 1\n"))),
        Some(nested_json(&format!("{json_fields}, \"extra\": 1"))),
        None,
        false,
    ));
    vectors.push(document(
        "nested missing key",
        &nested,
        Some(nested_toml("tag = 7\nhead = [true, -2]\nbody = [258, \"0xffff\"]\n")),
        Some(nested_json("\"tag\": 7, \"head\": [true, -2], \"body\": [258, \"0xffff\"]")),
        None,
        false,
    ));
    vectors.push(document(
        "nested duplicate key",
        &nested,
        None,
        Some(nested_json(&format!("\"tag\": 7, {json_fields}"))),
        None,
        false,
    ));
    for length in [nested_bytes.len() - 1, nested_bytes.len() + 1] {
        let mut bytes = nested_bytes.clone();
        bytes.resize(length, 0);
        let name = format!("nested blob of {length} bytes");
        vectors.push(document(&name, &nested, None, None, Some(bytes), false));
    }

    // An array of structs is an array of tables in TOML.
    let cells = abi(
        vec![(
            "x",
            AbiType::Array {
                length: 2,
                typ: Box::new(AbiType::Struct {
                    path: "Cell".into(),
                    fields: vec![("b".into(), unsigned(8)), ("a".into(), AbiType::Boolean)],
                }),
            },
        )],
        None,
    );
    vectors.push(document(
        "array of tables",
        &cells,
        Some("[[x]]\nb = 1\na = true\n\n[[x]]\nb = 2\na = false\n".into()),
        Some("{\"x\": [{\"b\": 1, \"a\": true}, {\"b\": 2, \"a\": false}]}".into()),
        Some(vec![0x01, 0x01, 0x02, 0x00]),
        true,
    ));

    // The top level: every parameter once, and `return` only when there is a return value,
    // whose blob is separate from the parameters'.
    let one = abi(vec![("x", unsigned(8))], None);
    let two = abi(vec![("x", unsigned(8)), ("y", unsigned(8))], None);
    let returning = abi(vec![("x", unsigned(8))], Some(unsigned(16)));
    for (name, abi, toml, json, accept) in [
        ("return", &returning, "x = 1\nreturn = 2", "{\"x\": 1, \"return\": 2}", true),
        (
            "return without a return value",
            &one,
            "x = 1\nreturn = 2",
            "{\"x\": 1, \"return\": 2}",
            false,
        ),
        ("unknown key", &one, "x = 1\ny = 2", "{\"x\": 1, \"y\": 2}", false),
        ("duplicate key", &one, "x = 1\nx = 1", "{\"x\": 1, \"x\": 1}", false),
        ("missing key", &two, "x = 1", "{\"x\": 1}", false),
    ] {
        let bytes = accept.then(|| vec![0x01]);
        vectors.push(document(name, abi, Some(toml.into()), Some(json.into()), bytes, accept));
    }
    vectors.push(document(
        "JSON array at the top level",
        &one,
        None,
        Some("[1]".into()),
        None,
        false,
    ));

    // Values both formats refuse, written the same in each.
    let pair = AbiType::Tuple { fields: vec![unsigned(8), unsigned(8)] };
    let array = AbiType::Array { length: 2, typ: Box::new(unsigned(8)) };
    for (name, typ, spelling) in [
        ("short tuple", pair.clone(), "[1]"),
        ("long tuple", pair, "[1, 2, 3]"),
        ("short array", array.clone(), "[1]"),
        ("long array", array, "[1, 2, 3]"),
        ("two-byte str<1>", AbiType::String { length: 1 }, "\"\u{e9}\""),
        ("quoted +", signed(8), "\"+5\""),
        ("quoted _", unsigned(16), "\"1_000\""),
        ("quoted 0X", signed(8), "\"0X10\""),
        ("quoted 0x alone", signed(8), "\"0x\""),
        ("quoted - alone", signed(8), "\"-\""),
        ("quoted --", signed(8), "\"--5\""),
        ("quoted text", unsigned(8), "\"five\""),
        ("quoted empty", unsigned(8), "\"\""),
        ("quoted leading space", unsigned(8), "\" 5\""),
        ("quoted trailing space", unsigned(8), "\"5 \""),
        ("quoted non-ASCII digit", unsigned(8), "\"\u{665}\""),
        ("quoted hex above i8::MAX", signed(8), "\"0x80\""),
        ("quoted -0x81 i8", signed(8), "\"-0x81\""),
        ("quoted -0 unsigned", unsigned(8), "\"-0\""),
        ("quoted -1 unsigned", unsigned(8), "\"-1\""),
        ("native -1 unsigned", unsigned(8), "-1"),
        ("native 256 u8", unsigned(8), "256"),
        ("native 128 i8", signed(8), "128"),
        ("native -129 i8", signed(8), "-129"),
        ("integer as bool", AbiType::Boolean, "1"),
        ("string as bool", AbiType::Boolean, "\"true\""),
        ("integer as string", AbiType::String { length: 1 }, "5"),
        ("bool as integer", unsigned(8), "true"),
    ] {
        vectors.push(value_vector(name, typ, spelling, None, false));
    }

    // Values only one format can spell.
    for (name, typ, spelling, bytes) in [
        ("TOML hex", unsigned(8), "0xff", Some(vec![0xff])),
        ("TOML separator", unsigned(16), "1_000", Some(vec![0xe8, 0x03])),
        ("TOML plus", signed(8), "+5", Some(vec![0x05])),
        ("TOML -0 signed", signed(8), "-0", Some(vec![0x00])),
        ("TOML -0 unsigned", unsigned(8), "-0", Some(vec![0x00])),
        ("TOML float", signed(8), "1.0", None),
        ("TOML -0.0", signed(8), "-0.0", None),
        ("TOML inf", signed(8), "inf", None),
        ("TOML nan", signed(8), "nan", None),
        ("TOML datetime", signed(8), "1979-05-27T07:32:00Z", None),
        ("TOML above i64::MAX", unsigned(64), "9223372036854775808", None),
    ] {
        let accept = bytes.is_some();
        vectors.push(ByteVector {
            name: name.into(),
            abi: single(typ),
            toml: Some(format!("x = {spelling}")),
            json: None,
            bytes,
            accept,
        });
    }
    for (name, typ, spelling, bytes) in [
        ("JSON -0 signed", signed(8), "-0", Some(vec![0x00])),
        ("JSON -0 unsigned", unsigned(8), "-0", Some(vec![0x00])),
        ("JSON -0.0", signed(8), "-0.0", Some(vec![0x00])),
        ("JSON -0e0", signed(8), "-0e0", Some(vec![0x00])),
        ("JSON negative underflow", signed(8), "-1e-400", Some(vec![0x00])),
        (
            "JSON i64::MAX",
            unsigned(64),
            "9223372036854775807",
            Some(le_bytes(&i64::MAX.into(), 64)),
        ),
        ("JSON 1e3", unsigned(64), "1e3", None),
        ("JSON 1.5", unsigned(64), "1.5", None),
        ("JSON 0.0", unsigned(64), "0.0", None),
        ("JSON positive underflow", unsigned(64), "1e-400", None),
        ("JSON above i64::MAX", unsigned(64), "9223372036854775808", None),
        ("JSON below i64::MIN", signed(128), "-9223372036854775809", None),
        ("JSON null", unsigned(64), "null", None),
    ] {
        let accept = bytes.is_some();
        vectors.push(ByteVector {
            name: name.into(),
            abi: single(typ),
            toml: None,
            json: Some(format!("{{\"x\": {spelling}}}")),
            bytes,
            accept,
        });
    }

    // Spellings both formats accept.
    for (name, typ, spelling, bytes) in [
        ("quoted -0 signed", signed(8), "\"-0\"", vec![0x00]),
        ("quoted -0x80", signed(8), "\"-0x80\"", vec![0x80]),
        ("quoted 0x7f i8", signed(8), "\"0x7f\"", vec![0x7f]),
        ("quoted mixed-case hex", unsigned(16), "\"0xAbC\"", vec![0xbc, 0x0a]),
        ("quoted leading zeros", unsigned(8), "\"007\"", vec![0x07]),
        ("native i64::MIN", signed(64), "-9223372036854775808", le_bytes(&i64::MIN.into(), 64)),
        ("bool false", AbiType::Boolean, "false", vec![0x00]),
    ] {
        vectors.push(value_vector(name, typ, spelling, Some(bytes), true));
    }

    // Blobs the decoder refuses.
    for (name, typ, bytes) in [
        ("u2 padding", unsigned(2), vec![0x04]),
        ("i17 padding", signed(17), vec![0x00, 0x00, 0x02]),
        ("bool 2", AbiType::Boolean, vec![0x02]),
        ("short i17", signed(17), vec![0xff, 0xff]),
        ("long i17", signed(17), vec![0xff, 0xff, 0x01, 0x00]),
        ("empty u8", unsigned(8), Vec::new()),
    ] {
        vectors.push(document(name, &single(typ), None, None, Some(bytes), false));
    }

    vectors
}

/// The `ceil(width / 8)` little-endian bytes of `value`'s `width`-bit pattern.
fn le_bytes(value: &BigInt, width: u32) -> Vec<u8> {
    let mut bytes = pattern(value, width).to_bytes_le();
    bytes.resize(width.div_ceil(8) as usize, 0);
    bytes
}

/// An ABI of private parameters and an optional private return value.
fn abi(parameters: Vec<(&str, AbiType)>, return_type: Option<AbiType>) -> Abi {
    Abi {
        parameters: parameters
            .into_iter()
            .map(|(name, typ)| AbiParameter {
                name: name.into(),
                typ,
                visibility: AbiVisibility::Private,
            })
            .collect(),
        return_type: return_type
            .map(|abi_type| AbiReturnType { abi_type, visibility: AbiVisibility::Private }),
        error_types: Default::default(),
    }
}

/// An ABI with one parameter `x` of type `typ`.
fn single(typ: AbiType) -> Abi {
    abi(vec![("x", typ)], None)
}

/// A case of one parameter `x`, spelled the same in both formats.
fn value_vector(
    name: &str,
    typ: AbiType,
    spelling: &str,
    bytes: Option<Vec<u8>>,
    accept: bool,
) -> ByteVector {
    ByteVector {
        name: name.into(),
        abi: single(typ),
        toml: Some(format!("x = {spelling}")),
        json: Some(format!("{{\"x\": {spelling}}}")),
        bytes,
        accept,
    }
}

fn document(
    name: &str,
    abi: &Abi,
    toml: Option<String>,
    json: Option<String>,
    bytes: Option<Vec<u8>>,
    accept: bool,
) -> ByteVector {
    ByteVector { name: name.into(), abi: abi.clone(), toml, json, bytes, accept }
}

#[cfg(test)]
mod tests {
    use acvm::{FieldConfig, FieldId};
    use num_bigint::BigUint;
    use strum::IntoEnumIterator;

    use crate::{
        Abi, AbiParameter, AbiVisibility, bytes, encode_scalar,
        input_parser::{Format, InputValue},
        scalar::carried_fields,
    };

    use super::{boundary_vectors, byte_vectors};

    #[test]
    fn the_parser_accepts_and_refuses_exactly_the_boundary_vectors() {
        for field in carried_fields() {
            for vector in boundary_vectors(field) {
                let abi = Abi {
                    parameters: vec![AbiParameter {
                        name: "x".into(),
                        typ: vector.typ.clone(),
                        visibility: AbiVisibility::Private,
                    }],
                    return_type: None,
                    error_types: Default::default(),
                };
                for format in Format::iter() {
                    let source = match format {
                        Format::Toml => format!("x = {}", vector.spelling),
                        Format::Json => format!("{{\"x\": {}}}", vector.spelling),
                    };
                    let element = format.parse(&source, &abi, field).ok().map(|mut inputs| {
                        let serialized = format.serialize(&inputs, &abi).unwrap();
                        assert_eq!(
                            format.parse(&serialized, &abi, field).unwrap(),
                            inputs,
                            "{}, {format:?}: {vector:?}",
                            field.name()
                        );
                        let value = inputs.remove("x").expect("x is parsed");
                        assert!(matches!(value, InputValue::Field(_)));
                        encode_scalar(&value, &vector.typ, field).expect("a parsed value encodes")
                    });
                    assert_eq!(element, vector.element, "{}, {format:?}: {vector:?}", field.name());
                }
            }
        }
    }

    #[test]
    fn the_codec_accepts_and_refuses_exactly_the_byte_vectors() {
        let vectors = byte_vectors();
        let names: std::collections::HashSet<_> =
            vectors.iter().map(|vector| &vector.name).collect();
        assert_eq!(names.len(), vectors.len(), "every byte vector has its own name");
        for vector in vectors {
            let (name, abi) = (&vector.name, &vector.abi);
            assert_eq!(bytes::check_abi(abi), Ok(()), "{name}");
            for (format, document) in [(Format::Toml, &vector.toml), (Format::Json, &vector.json)] {
                let Some(document) = document else { continue };
                let parsed = bytes::parse(&format, document, abi);
                if !vector.accept {
                    assert!(parsed.is_err(), "{name}, {format:?}: {parsed:?}");
                    continue;
                }
                let inputs = parsed.unwrap_or_else(|error| panic!("{name}, {format:?}: {error}"));
                let serialized = bytes::serialize(&format, abi, &inputs).unwrap();
                assert_eq!(
                    bytes::parse(&format, &serialized, abi).as_ref(),
                    Ok(&inputs),
                    "{name}, {format:?}"
                );
                if let Some(blob) = &vector.bytes {
                    let encoded = bytes::encode_params(abi, &inputs.params);
                    assert_eq!(encoded.as_ref(), Ok(blob), "{name}, {format:?}");
                }
            }
            if let Some(blob) = &vector.bytes {
                let decoded = bytes::decode_params(abi, blob);
                if vector.accept {
                    let params = decoded.unwrap_or_else(|error| panic!("{name}: {error}"));
                    assert_eq!(bytes::encode_params(abi, &params).as_ref(), Ok(blob), "{name}");
                } else {
                    assert!(decoded.is_err(), "{name}: {decoded:?}");
                }
            }
        }
    }

    /// The vectors differ between fields only where the modulus decides.
    #[test]
    fn the_goldilocks_vectors_refuse_the_64_bit_patterns_at_or_above_its_modulus() {
        let goldilocks = boundary_vectors(FieldConfig::new(FieldId::Goldilocks));
        let refused_i64: Vec<_> = goldilocks
            .iter()
            .filter(|vector| {
                vector.typ == crate::AbiType::Integer { sign: crate::Sign::Signed, width: 64 }
                    && vector.element.is_none()
            })
            .map(|vector| vector.spelling.trim_matches('"').to_string())
            .collect();
        assert_eq!(refused_i64, ["-4294967295", "-4294967295", "-1", "-1"]);

        let bn254 = boundary_vectors(FieldConfig::new(FieldId::Bn254));
        let bn254_u64_elements: Vec<_> = bn254
            .iter()
            .filter(|vector| {
                vector.typ == crate::AbiType::Integer { sign: crate::Sign::Unsigned, width: 64 }
            })
            .map(|vector| vector.element.clone())
            .collect();
        assert!(bn254_u64_elements.iter().all(Option::is_some));
        assert_eq!(bn254_u64_elements.last(), Some(&Some(BigUint::from(u64::MAX))));
    }
}
