//! The byte codec of entry-point values, the same under every field.
//!
//! A value of a field-free ABI type is carried as little-endian bytes: an integer of width `w`
//! as its `w`-bit two's complement pattern in `ceil(w / 8)` bytes, with the padding bits above
//! the width zero; a `bool` as one byte, `0` or `1`; a `str<N>` as its `N` raw bytes. Arrays,
//! tuples and structs hold their elements in declaration order, with no alignment and no length
//! prefix. The parameters form one blob and the return value its own. A `Field` has no byte
//! encoding, so an ABI that names one is refused, as is one that names an empty value, which no
//! entry point takes or returns.
//!
//! Inputs are read from TOML or JSON by a strict parser: shapes are exact, an integer is a
//! native integer of the format or a quoted `-?(0x[0-9a-fA-F]+|[0-9]+)`, and a boolean is
//! `true` or `false`. The field-element path ([`crate::input_parser`], [`crate::Abi::encode`])
//! is separate and unchanged.

use std::fmt;

use num_bigint::BigInt;
use thiserror::Error;

use crate::{AbiType, Sign};

mod codec;
mod layout;
#[cfg(test)]
mod tests;
mod text;

pub use codec::{decode_params, decode_return, encode_params, encode_return};
pub use layout::{Layout, byte_size, check_abi, layout};
pub use text::{parse, serialize};

/// The version of the byte encoding, changed whenever the bytes of some value change.
pub const BYTE_ENCODING_VERSION: u32 = 1;

/// The widest integer type with a byte encoding: `u16384` and `i16384`. The narrowest is 2 bits
/// wide; width 1 has no encoding.
pub const MAX_INTEGER_WIDTH: u32 = 16384;

/// The exclusive bound on the length of each blob: the parameters together, and the return
/// value, each take fewer than 2^32 bytes.
pub const MAX_INPUT_BYTES: u64 = 1 << 32;

/// A value of a field-free ABI type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// An integer, signed or unsigned.
    Int(BigInt),
    /// A boolean.
    Bool(bool),
    /// The raw bytes of a `str<N>`, which need not be UTF-8.
    Str(Vec<u8>),
    /// The elements of an array or a tuple, or the fields of a struct, in declaration order.
    Seq(Vec<Value>),
}

impl Value {
    fn kind(&self) -> &'static str {
        match self {
            Value::Int(_) => "an integer",
            Value::Bool(_) => "a boolean",
            Value::Str(_) => "a string",
            Value::Seq(_) => "a sequence",
        }
    }
}

/// The kind of [`Value`] that holds a value of `typ`, for errors.
fn value_kind(typ: &AbiType) -> &'static str {
    match typ {
        AbiType::Field | AbiType::Integer { .. } => "an integer",
        AbiType::Boolean => "a boolean",
        AbiType::String { .. } => "a string",
        AbiType::Array { .. } | AbiType::Tuple { .. } | AbiType::Struct { .. } => "a sequence",
    }
}

/// The values read from an input document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Inputs {
    /// The value of each parameter, in declaration order.
    pub params: Vec<Value>,
    /// The return value, or `None` when the document gives none.
    pub ret: Option<Value>,
}

/// Why an ABI, a document or a blob was refused. Each variant but [`Error::Syntax`] names where,
/// as a path such as `x[2].head.0`: a parameter or `return`, then `[i]` for an array element,
/// `.name` for a struct field and `.i` for a tuple element. Where a type rather than a value is
/// refused, `[_]` stands for every element of an array. An empty path is the whole blob or
/// document.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Error {
    /// The text is not TOML or JSON, whose parser's message gives the line and column. A
    /// repeated TOML key is refused here. Also returned if a document cannot be written, which
    /// does not happen for inputs that pass the other checks.
    #[error("cannot read the input: {message}")]
    Syntax { message: String },
    /// A JSON object names a key twice.
    #[error("{}the key is given more than once", at(path))]
    DuplicateKey { path: String },
    /// A key names no parameter or struct field, or is `return` when there is no return value.
    #[error("{}unknown key", at(path))]
    UnknownKey { path: String },
    /// A parameter or struct field has no value.
    #[error("{}missing value", at(path))]
    Missing { path: String },
    /// The ABI names a type with no byte encoding: a `Field`, an integer width outside
    /// `2..=MAX_INTEGER_WIDTH`, or an empty array, string, tuple or struct.
    #[error("{}`{}` has no byte encoding", at(path), type_name(typ))]
    UnsupportedType { path: String, typ: AbiType },
    /// The ABI declares two parameters, or two fields of a struct, with the same name.
    #[error("{}the name is declared more than once", at(path))]
    DuplicateName { path: String },
    /// The ABI declares a parameter named `return`, the key of the return value.
    #[error("{}a parameter cannot be named `return`", at(path))]
    ReservedName { path: String },
    /// A value is not of the kind its type takes.
    #[error("{}expected {expected}, found {found}", at(path))]
    Shape { path: String, expected: &'static str, found: &'static str },
    /// An array or tuple has the wrong number of elements, a struct the wrong number of fields,
    /// a `str<N>` other than `N` bytes, or a list of parameter values other than one per
    /// parameter.
    #[error("{}expected length {expected}, found {found}", at(path))]
    Length { path: String, expected: usize, found: usize },
    /// An integer is written in a spelling the parser refuses.
    #[error("{}cannot read `{text}`: {rule}", at(path))]
    Spelling { path: String, text: String, rule: &'static str },
    /// An integer is outside the range of its type.
    #[error("{}{value} is out of range for `{}`", at(path), type_name(typ))]
    OutOfRange { path: String, value: BigInt, typ: AbiType },
    /// An integer's bytes set a padding bit above its width.
    #[error("{}a padding bit above the integer's width is set", at(path))]
    Padding { path: String },
    /// A boolean's byte is neither `0` nor `1`.
    #[error("{}byte {byte} is not a boolean", at(path))]
    BadBool { path: String, byte: u8 },
    /// A blob is not exactly as long as its layout.
    #[error("{}expected {expected} bytes, found {found}", at(path))]
    ByteLength { path: String, expected: usize, found: usize },
    /// A blob or a type's encoding would reach [`MAX_INPUT_BYTES`].
    #[error("{}the byte encoding reaches 2^32 bytes", at(path))]
    TooLarge { path: String },
    /// A `str<N>` holds bytes that are not UTF-8, which have no text spelling.
    #[error("{}the string is not UTF-8", at(path))]
    NotUtf8 { path: String },
}

/// The prefix that names a path in a message, or nothing for the empty path.
fn at(path: &str) -> String {
    if path.is_empty() { String::new() } else { format!("`{path}`: ") }
}

/// A type as Noir spells it, for messages.
fn type_name(typ: &AbiType) -> String {
    match typ {
        AbiType::Field => "Field".to_string(),
        AbiType::Integer { sign: Sign::Unsigned, width } => format!("u{width}"),
        AbiType::Integer { sign: Sign::Signed, width } => format!("i{width}"),
        AbiType::Boolean => "bool".to_string(),
        AbiType::String { length } => format!("str<{length}>"),
        AbiType::Array { length, typ } => format!("[{}; {length}]", type_name(typ)),
        AbiType::Tuple { fields } => {
            let fields: Vec<_> = fields.iter().map(type_name).collect();
            match fields.as_slice() {
                [field] => format!("({field},)"),
                fields => format!("({})", fields.join(", ")),
            }
        }
        AbiType::Struct { path, .. } => path.clone(),
    }
}

/// Where a value or type sits, rendered only when an error names it.
#[derive(Clone, Copy)]
enum Path<'a> {
    /// A parameter, `return`, or the empty root of a whole blob, document or bare type.
    Root(&'a str),
    /// An element of an array.
    Index(&'a Path<'a>, usize),
    /// Every element of an array, where a type rather than a value is checked.
    Each(&'a Path<'a>),
    /// A field of a struct.
    Field(&'a Path<'a>, &'a str),
    /// An element of a tuple.
    Position(&'a Path<'a>, usize),
}

impl Path<'_> {
    fn member(
        f: &mut fmt::Formatter<'_>,
        parent: &Path<'_>,
        name: &dyn fmt::Display,
    ) -> fmt::Result {
        if matches!(parent, Path::Root("")) {
            write!(f, "{name}")
        } else {
            write!(f, "{parent}.{name}")
        }
    }
}

impl fmt::Display for Path<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Path::Root(name) => f.write_str(name),
            Path::Index(parent, index) => write!(f, "{parent}[{index}]"),
            Path::Each(parent) => write!(f, "{parent}[_]"),
            Path::Field(parent, name) => Path::member(f, parent, name),
            Path::Position(parent, index) => Path::member(f, parent, index),
        }
    }
}

/// The error for a type with no byte encoding.
fn unsupported(typ: &AbiType, path: Path<'_>) -> Error {
    Error::UnsupportedType { path: path.to_string(), typ: typ.clone() }
}

/// The error for an array, tuple, struct or string of `found` elements or bytes, unless that is
/// the `expected` number.
fn check_length(expected: usize, found: usize, path: Path<'_>) -> Result<(), Error> {
    if expected == found {
        Ok(())
    } else {
        Err(Error::Length { path: path.to_string(), expected, found })
    }
}
