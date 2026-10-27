//! The strict TOML and JSON reader and writer of entry-point inputs.

use std::collections::HashMap;
use std::fmt;

use num_bigint::{BigInt, BigUint};
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, Serializer};

use crate::{Abi, AbiType, MAIN_RETURN_NAME, input_parser::Format};

use super::{
    Error, Inputs, MAX_INTEGER_WIDTH, Path, Value, check_length, codec::check_int, layout::layout,
    unsupported, value_kind,
};

const SPELLING: &str =
    "a quoted integer is an optional -, an optional 0x and an optional +, then digits and _";
const NATIVE_RANGE: &str = "a native integer is at most i64::MAX; quote a larger one";
const DIGIT_LIMIT: &str = "a quoted integer has at most 16384 digits after its leading zeros";

/// Reads the inputs of `abi` from a TOML or JSON document.
///
/// The document holds one value per parameter, keyed by name, and optionally a value keyed
/// `return` when `abi` has a return value. As in Noir's input parser, a key that names no
/// parameter or struct field, or `return` when `abi` has no return value, is ignored, and a key
/// repeated in JSON takes its last value (TOML refuses a repeated key itself). Otherwise shapes are
/// exact: a missing key, an array or tuple of the wrong length and a string of the wrong byte
/// length are refused. An integer is a native integer of the format (TOML takes every spelling it has) or a
/// quoted integer as Noir's input parser reads it: an optional `-`, an optional `0x` and an
/// optional `+`, then digits that may hold `_` after the first. A negative value is out of range
/// for an unsigned type, but `"-0"` is 0. A boolean is `true` or `false`, or 0 or 1 spelled as an
/// integer, as Noir's input parser also takes it.
/// Floats, datetimes and `null` are refused, except JSON's negative zero: a native
/// `-0`, which `serde_json` reads as the float `-0.0`, is the integer 0, and so are the JSON
/// spellings `-0.0`, `-0e0` and a negative underflow such as `-1e-400`. Positive zero (`0.0`)
/// is refused.
pub fn parse(format: &Format, src: &str, abi: &Abi) -> Result<Inputs, Error> {
    layout(abi)?;
    let (root, reader) = match format {
        Format::Toml => {
            let table = toml::from_str::<toml::Table>(src).map_err(syntax)?;
            (Node::from(toml::Value::Table(table)), Reader { map: "a table" })
        }
        Format::Json => {
            (serde_json::from_str::<Node>(src).map_err(syntax)?, Reader { map: "an object" })
        }
    };
    let root_path = Path::Root("");
    let entries = match root {
        Node::Map(entries) => entries,
        node => return Err(reader.shape(root_path, reader.map, &node)),
    };

    let has_return = abi.return_type.is_some();
    let mut values = keyed(entries, |key| {
        abi.parameters.iter().any(|param| param.name == key)
            || (has_return && key == MAIN_RETURN_NAME)
    });
    let params = abi
        .parameters
        .iter()
        .map(|param| {
            let path = Path::Root(&param.name);
            let node = values.remove(param.name.as_str()).ok_or_else(|| missing(path))?;
            reader.read(&param.typ, node, path)
        })
        .collect::<Result<_, _>>()?;
    let ret = match (&abi.return_type, values.remove(MAIN_RETURN_NAME)) {
        (Some(ret), Some(node)) => {
            Some(reader.read(&ret.abi_type, node, Path::Root(MAIN_RETURN_NAME))?)
        }
        _ => None,
    };
    Ok(Inputs { params, ret })
}

/// Writes `inputs` as a TOML or JSON document that [`parse`] reads back: integers as quoted
/// decimal, and parameters and struct fields in declaration order with `return` last when given,
/// except that TOML writes a table after the plain values beside it. A `str<N>` that is not UTF-8
/// has no spelling and is refused.
pub fn serialize(format: &Format, abi: &Abi, inputs: &Inputs) -> Result<String, Error> {
    layout(abi)?;
    check_length(abi.parameters.len(), inputs.params.len(), Path::Root(""))?;
    let mut entries = Vec::with_capacity(abi.parameters.len() + 1);
    for (param, value) in abi.parameters.iter().zip(&inputs.params) {
        entries.push((param.name.as_str(), write(&param.typ, value, Path::Root(&param.name))?));
    }
    if let Some(ret) = &inputs.ret {
        let path = Path::Root(MAIN_RETURN_NAME);
        let Some(typ) = &abi.return_type else {
            return Err(Error::UnexpectedReturn { path: path.to_string() });
        };
        entries.push((MAIN_RETURN_NAME, write(&typ.abi_type, ret, path)?));
    }
    let document = Out::Map(entries);
    match format {
        Format::Toml => toml::to_string(&document).map_err(syntax),
        Format::Json => serde_json::to_string(&document).map_err(syntax),
    }
}

fn syntax(error: impl fmt::Display) -> Error {
    Error::Syntax { message: error.to_string() }
}

fn missing(path: Path<'_>) -> Error {
    Error::Missing { path: path.to_string() }
}

/// A value as a document spells it, before it is read against its type.
enum Node {
    Int(i64),
    /// A JSON integer above `i64::MAX`, which neither format carries as a native integer.
    Wide(u64),
    Bool(bool),
    Str(String),
    Seq(Vec<Node>),
    /// The entries of a table or object: an object's in document order, a repeated key kept, a
    /// table's sorted by key.
    Map(Vec<(String, Node)>),
    /// A value no input type takes, named for errors: a float, a datetime or `null`.
    Other(&'static str),
}

impl From<toml::Value> for Node {
    fn from(value: toml::Value) -> Self {
        match value {
            toml::Value::String(text) => Node::Str(text),
            toml::Value::Integer(value) => Node::Int(value),
            toml::Value::Float(_) => Node::Other("a float"),
            toml::Value::Boolean(value) => Node::Bool(value),
            toml::Value::Datetime(_) => Node::Other("a datetime"),
            toml::Value::Array(elements) => {
                Node::Seq(elements.into_iter().map(Node::from).collect())
            }
            toml::Value::Table(table) => {
                Node::Map(table.into_iter().map(|(key, value)| (key, Node::from(value))).collect())
            }
        }
    }
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(NodeVisitor)
    }
}

/// Reads every level of a JSON document, the top-level object included, into a [`Node`]. A
/// value no input type takes is kept as [`Node::Wide`] or [`Node::Other`], so that the reader
/// refuses it with the path of the value it stands for.
struct NodeVisitor;

impl<'de> Visitor<'de> for NodeVisitor {
    type Value = Node;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Node, E> {
        Ok(Node::Bool(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Node, E> {
        Ok(Node::Int(value))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Node, E> {
        Ok(i64::try_from(value).map_or(Node::Wide(value), Node::Int))
    }

    /// Every float is refused but negative zero, which is the integer 0.
    ///
    /// No `i64` is negative zero, so `serde_json` reads the JSON integer `-0` as the float
    /// `-0.0`, and this is how a native `-0` reads as 0. The same float is read from the JSON
    /// spellings `-0.0` and `-0e0`, and from a negative number too small to represent, such as
    /// `-1e-400`; those read as 0 as well. Positive zero, `0.0`, is a float and is refused.
    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Node, E> {
        Ok(if value == 0.0 && value.is_sign_negative() {
            Node::Int(0)
        } else {
            Node::Other("a float")
        })
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Node, E> {
        Ok(Node::Str(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Node, E> {
        Ok(Node::Str(value))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Node, E> {
        Ok(Node::Other("null"))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Node, A::Error> {
        let mut elements = Vec::new();
        while let Some(element) = seq.next_element()? {
            elements.push(element);
        }
        Ok(Node::Seq(elements))
    }

    /// Keeps every entry in document order, so that the reader takes a repeated key's last value.
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Node, A::Error> {
        let mut entries: Vec<(String, Node)> = Vec::new();
        while let Some(entry) = map.next_entry()? {
            entries.push(entry);
        }
        Ok(Node::Map(entries))
    }
}

/// The entries of a table or object that `known` takes, by key. As in Noir's input parser, the
/// others are ignored and a repeated key takes its last value.
fn keyed(entries: Vec<(String, Node)>, known: impl Fn(&str) -> bool) -> HashMap<String, Node> {
    entries.into_iter().filter(|(key, _)| known(key)).collect()
}

/// Reads nodes against their types. `map` names a table or an object in errors.
struct Reader {
    map: &'static str,
}

impl Reader {
    fn read(&self, typ: &AbiType, node: Node, path: Path<'_>) -> Result<Value, Error> {
        match (typ, node) {
            (AbiType::Field, _) => Err(unsupported(typ, path)),
            (AbiType::Integer { sign, width }, node) => {
                let value = self.read_int(node, path)?;
                check_int(&value, *sign, *width, path)?;
                Ok(Value::Int(value))
            }
            (AbiType::Boolean, Node::Bool(value)) => Ok(Value::Bool(value)),
            (AbiType::Boolean, node @ (Node::Int(_) | Node::Wide(_) | Node::Str(_))) => {
                let value = self.read_int(node, path)?;
                match u8::try_from(&value) {
                    Ok(0) => Ok(Value::Bool(false)),
                    Ok(1) => Ok(Value::Bool(true)),
                    _ => Err(Error::OutOfRange { path: path.to_string(), value, typ: typ.clone() }),
                }
            }
            (AbiType::String { length }, Node::Str(text)) => {
                check_length(*length as usize, text.len(), path)?;
                Ok(Value::Str(text.into_bytes()))
            }
            (AbiType::Array { length, typ }, Node::Seq(elements)) => {
                check_length(*length as usize, elements.len(), path)?;
                elements
                    .into_iter()
                    .enumerate()
                    .map(|(index, element)| self.read(typ, element, Path::Index(&path, index)))
                    .collect::<Result<_, _>>()
                    .map(Value::Seq)
            }
            (AbiType::Tuple { fields }, Node::Seq(elements)) => {
                check_length(fields.len(), elements.len(), path)?;
                fields
                    .iter()
                    .zip(elements)
                    .enumerate()
                    .map(|(index, (typ, element))| {
                        self.read(typ, element, Path::Position(&path, index))
                    })
                    .collect::<Result<_, _>>()
                    .map(Value::Seq)
            }
            (AbiType::Struct { fields, .. }, Node::Map(entries)) => {
                let mut values = keyed(entries, |key| fields.iter().any(|(name, _)| name == key));
                fields
                    .iter()
                    .map(|(name, typ)| {
                        let field_path = Path::Field(&path, name);
                        let node =
                            values.remove(name.as_str()).ok_or_else(|| missing(field_path))?;
                        self.read(typ, node, field_path)
                    })
                    .collect::<Result<_, _>>()
                    .map(Value::Seq)
            }
            (typ, node) => Err(self.shape(path, self.expected(typ), &node)),
        }
    }

    fn read_int(&self, node: Node, path: Path<'_>) -> Result<BigInt, Error> {
        match node {
            Node::Int(value) => Ok(BigInt::from(value)),
            Node::Str(text) => {
                scan(&text).map_err(|rule| Error::Spelling { path: path.to_string(), text, rule })
            }
            Node::Wide(value) => Err(Error::Spelling {
                path: path.to_string(),
                text: value.to_string(),
                rule: NATIVE_RANGE,
            }),
            node => Err(self.shape(path, "an integer", &node)),
        }
    }

    fn shape(&self, path: Path<'_>, expected: &'static str, node: &Node) -> Error {
        Error::Shape { path: path.to_string(), expected, found: self.kind(node) }
    }

    /// The kind of node that spells a value of `typ`.
    fn expected(&self, typ: &AbiType) -> &'static str {
        match typ {
            AbiType::Field | AbiType::Integer { .. } => "an integer",
            AbiType::Boolean => "a boolean",
            AbiType::String { .. } => "a string",
            AbiType::Array { .. } | AbiType::Tuple { .. } => "an array",
            AbiType::Struct { .. } => self.map,
        }
    }

    fn kind(&self, node: &Node) -> &'static str {
        match node {
            Node::Int(_) | Node::Wide(_) => "an integer",
            Node::Bool(_) => "a boolean",
            Node::Str(_) => "a string",
            Node::Seq(_) => "an array",
            Node::Map(_) => self.map,
            Node::Other(kind) => kind,
        }
    }
}

/// Reads a quoted integer as Noir's input parser does, or names the rule `text` breaks: an
/// optional `-`, then `0x` and hex digits or decimal digits, which may start with one `+` and
/// hold `_` after the first digit. The `-` is read for every type, and the range check refuses a
/// negative value of an unsigned type, so `"-0"` is 0 there. A number of more significant digits
/// than the widest type has bits is refused before it is converted, which would take time
/// quadratic in its length.
fn scan(text: &str) -> Result<BigInt, &'static str> {
    let (negative, unsigned) = match text.strip_prefix('-') {
        Some(unsigned) => (true, unsigned),
        None => (false, text),
    };
    let (digits, radix) = match unsigned.strip_prefix("0x") {
        Some(digits) => (digits, 16),
        None => (unsigned, 10),
    };
    let digits = digits.strip_prefix('+').unwrap_or(digits);
    if digits.is_empty()
        || digits.starts_with('_')
        || !digits.chars().all(|digit| digit == '_' || digit.is_digit(radix))
    {
        return Err(SPELLING);
    }
    let significant = digits.trim_start_matches(['0', '_']).bytes().filter(|&digit| digit != b'_');
    if significant.count() > MAX_INTEGER_WIDTH as usize {
        return Err(DIGIT_LIMIT);
    }
    let magnitude = BigUint::parse_bytes(digits.as_bytes(), radix).ok_or(SPELLING)?;
    let magnitude = BigInt::from(magnitude);
    Ok(if negative { -magnitude } else { magnitude })
}

/// A document as it is written, with each integer already quoted decimal text.
enum Out<'a> {
    Text(String),
    Bool(bool),
    Seq(Vec<Out<'a>>),
    Map(Vec<(&'a str, Out<'a>)>),
}

impl Serialize for Out<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Out::Text(text) => serializer.serialize_str(text),
            Out::Bool(value) => serializer.serialize_bool(*value),
            Out::Seq(elements) => serializer.collect_seq(elements),
            Out::Map(entries) => {
                serializer.collect_map(entries.iter().map(|(key, value)| (key, value)))
            }
        }
    }
}

fn write<'a>(typ: &'a AbiType, value: &Value, path: Path<'_>) -> Result<Out<'a>, Error> {
    match (typ, value) {
        (AbiType::Field, _) => Err(unsupported(typ, path)),
        (AbiType::Integer { sign, width }, Value::Int(value)) => {
            check_int(value, *sign, *width, path)?;
            Ok(Out::Text(value.to_string()))
        }
        (AbiType::Boolean, Value::Bool(value)) => Ok(Out::Bool(*value)),
        (AbiType::String { length }, Value::Str(bytes)) => {
            check_length(*length as usize, bytes.len(), path)?;
            let text = std::str::from_utf8(bytes)
                .map_err(|_| Error::NotUtf8 { path: path.to_string() })?;
            Ok(Out::Text(text.to_owned()))
        }
        (AbiType::Array { length, typ }, Value::Seq(elements)) => {
            check_length(*length as usize, elements.len(), path)?;
            elements
                .iter()
                .enumerate()
                .map(|(index, element)| write(typ, element, Path::Index(&path, index)))
                .collect::<Result<_, _>>()
                .map(Out::Seq)
        }
        (AbiType::Tuple { fields }, Value::Seq(elements)) => {
            check_length(fields.len(), elements.len(), path)?;
            fields
                .iter()
                .zip(elements)
                .enumerate()
                .map(|(index, (typ, element))| write(typ, element, Path::Position(&path, index)))
                .collect::<Result<_, _>>()
                .map(Out::Seq)
        }
        (AbiType::Struct { fields, .. }, Value::Seq(elements)) => {
            check_length(fields.len(), elements.len(), path)?;
            fields
                .iter()
                .zip(elements)
                .map(|((name, typ), element)| {
                    Ok((name.as_str(), write(typ, element, Path::Field(&path, name))?))
                })
                .collect::<Result<_, _>>()
                .map(Out::Map)
        }
        (typ, value) => Err(Error::Shape {
            path: path.to_string(),
            expected: value_kind(typ),
            found: value.kind(),
        }),
    }
}
