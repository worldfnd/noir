//! The encoder and decoder of the parameter blob and the return blob.

use num_bigint::{BigInt, BigUint};
use num_traits::{One, Zero};

use crate::{Abi, AbiType, MAIN_RETURN_NAME, Sign, scalar::signed_value};

use super::{Error, Path, Value, check_length, layout::layout, unsupported, value_kind};

/// The parameter blob of `params`, one value per parameter of `abi` in declaration order.
pub fn encode_params(abi: &Abi, params: &[Value]) -> Result<Vec<u8>, Error> {
    layout(abi)?;
    check_length(abi.parameters.len(), params.len(), Path::Root(""))?;
    let mut blob = Vec::new();
    for (param, value) in abi.parameters.iter().zip(params) {
        encode(&param.typ, value, Path::Root(&param.name), &mut blob)?;
    }
    Ok(blob)
}

/// The return blob of `ret`, which is separate from the parameter blob.
pub fn encode_return(abi: &Abi, ret: &Value) -> Result<Vec<u8>, Error> {
    layout(abi)?;
    let path = Path::Root(MAIN_RETURN_NAME);
    let Some(typ) = &abi.return_type else {
        return Err(Error::UnexpectedReturn { path: path.to_string() });
    };
    let mut blob = Vec::new();
    encode(&typ.abi_type, ret, path, &mut blob)?;
    Ok(blob)
}

/// The parameters of `abi` held in `bytes`, which must be exactly the parameter blob's length.
/// The length is checked before any value is decoded.
pub fn decode_params(abi: &Abi, bytes: &[u8]) -> Result<Vec<Value>, Error> {
    let layout = layout(abi)?;
    if bytes.len() != layout.params_len {
        return Err(Error::ByteLength {
            path: String::new(),
            expected: layout.params_len,
            found: bytes.len(),
        });
    }
    let mut input = bytes;
    abi.parameters
        .iter()
        .map(|param| decode(&param.typ, &mut input, Path::Root(&param.name)))
        .collect()
}

/// The return value of `abi` held in `bytes`, which must be exactly the return blob's length.
/// The length is checked before any value is decoded.
pub fn decode_return(abi: &Abi, bytes: &[u8]) -> Result<Value, Error> {
    let layout = layout(abi)?;
    let path = Path::Root(MAIN_RETURN_NAME);
    let (Some(typ), Some(expected)) = (&abi.return_type, layout.ret_len) else {
        return Err(Error::UnexpectedReturn { path: path.to_string() });
    };
    if bytes.len() != expected {
        return Err(Error::ByteLength { path: path.to_string(), expected, found: bytes.len() });
    }
    let mut input = bytes;
    decode(&typ.abi_type, &mut input, path)
}

fn encode(typ: &AbiType, value: &Value, path: Path<'_>, blob: &mut Vec<u8>) -> Result<(), Error> {
    match (typ, value) {
        (AbiType::Field, _) => return Err(unsupported(typ, path)),
        (AbiType::Integer { sign, width }, Value::Int(value)) => {
            check_int(value, *sign, *width, path)?;
            push_int(value, *width, blob);
        }
        (AbiType::Boolean, Value::Bool(value)) => blob.push(u8::from(*value)),
        (AbiType::String { length }, Value::Str(bytes)) => {
            check_length(*length as usize, bytes.len(), path)?;
            blob.extend_from_slice(bytes);
        }
        (AbiType::Array { length, typ }, Value::Seq(elements)) => {
            check_length(*length as usize, elements.len(), path)?;
            for (index, element) in elements.iter().enumerate() {
                encode(typ, element, Path::Index(&path, index), blob)?;
            }
        }
        (AbiType::Tuple { fields }, Value::Seq(elements)) => {
            check_length(fields.len(), elements.len(), path)?;
            for (index, (typ, element)) in fields.iter().zip(elements).enumerate() {
                encode(typ, element, Path::Position(&path, index), blob)?;
            }
        }
        (AbiType::Struct { fields, .. }, Value::Seq(elements)) => {
            check_length(fields.len(), elements.len(), path)?;
            for ((name, typ), element) in fields.iter().zip(elements) {
                encode(typ, element, Path::Field(&path, name), blob)?;
            }
        }
        (typ, value) => {
            return Err(Error::Shape {
                path: path.to_string(),
                expected: value_kind(typ),
                found: value.kind(),
            });
        }
    }
    Ok(())
}

fn decode(typ: &AbiType, input: &mut &[u8], path: Path<'_>) -> Result<Value, Error> {
    let value = match typ {
        AbiType::Field => return Err(unsupported(typ, path)),
        AbiType::Integer { sign, width } => {
            let bytes = take(input, width.div_ceil(8) as usize, path)?;
            let pattern = BigUint::from_bytes_le(bytes);
            if pattern.bits() > u64::from(*width) {
                return Err(Error::Padding { path: path.to_string() });
            }
            Value::Int(match sign {
                Sign::Unsigned => BigInt::from(pattern),
                Sign::Signed => signed_value(&pattern, *width),
            })
        }
        AbiType::Boolean => match take(input, 1, path)?[0] {
            0 => Value::Bool(false),
            1 => Value::Bool(true),
            byte => return Err(Error::BadBool { path: path.to_string(), byte }),
        },
        AbiType::String { length } => Value::Str(take(input, *length as usize, path)?.to_vec()),
        AbiType::Array { length, typ } => {
            let length = *length as usize;
            let mut elements = Vec::with_capacity(length);
            for index in 0..length {
                elements.push(decode(typ, input, Path::Index(&path, index))?);
            }
            Value::Seq(elements)
        }
        AbiType::Tuple { fields } => Value::Seq(
            fields
                .iter()
                .enumerate()
                .map(|(index, typ)| decode(typ, input, Path::Position(&path, index)))
                .collect::<Result<_, _>>()?,
        ),
        AbiType::Struct { fields, .. } => Value::Seq(
            fields
                .iter()
                .map(|(name, typ)| decode(typ, input, Path::Field(&path, name)))
                .collect::<Result<_, _>>()?,
        ),
    };
    Ok(value)
}

/// The next `count` bytes of `input`. The blob's length is checked against its layout first, so
/// this never runs short.
fn take<'a>(input: &mut &'a [u8], count: usize, path: Path<'_>) -> Result<&'a [u8], Error> {
    let (taken, rest) = input.split_at_checked(count).ok_or_else(|| Error::ByteLength {
        path: path.to_string(),
        expected: count,
        found: input.len(),
    })?;
    *input = rest;
    Ok(taken)
}

/// Refuses an integer outside the range of its type, whatever its width.
pub(super) fn check_int(
    value: &BigInt,
    sign: Sign,
    width: u32,
    path: Path<'_>,
) -> Result<(), Error> {
    let (min, max) = match sign {
        Sign::Unsigned => (BigInt::zero(), two_pow(width) - 1),
        Sign::Signed => {
            let half = two_pow(width - 1);
            (-half.clone(), half - 1)
        }
    };
    if *value < min || *value > max {
        return Err(Error::OutOfRange {
            path: path.to_string(),
            value: value.clone(),
            typ: AbiType::Integer { sign, width },
        });
    }
    Ok(())
}

fn two_pow(bits: u32) -> BigInt {
    BigInt::one() << bits
}

/// Appends the `width`-bit two's complement pattern of `value`, which is in range, in
/// `ceil(width / 8)` little-endian bytes with the padding bits zero.
fn push_int(value: &BigInt, width: u32, blob: &mut Vec<u8>) {
    let length = width.div_ceil(8) as usize;
    let mut bytes = value.to_signed_bytes_le();
    let sign_extension = if value.sign() == num_bigint::Sign::Minus { 0xff } else { 0 };
    bytes.resize(length, sign_extension);
    if width % 8 != 0 {
        bytes[length - 1] &= (1 << (width % 8)) - 1;
    }
    blob.extend_from_slice(&bytes);
}
