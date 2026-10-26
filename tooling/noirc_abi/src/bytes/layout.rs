//! The byte size of each type and where each parameter sits in the parameter blob.

use std::collections::HashSet;
use std::ops::Range;

use crate::{Abi, AbiType, MAIN_RETURN_NAME};

use super::{Error, MAX_INPUT_BYTES, MAX_INTEGER_WIDTH, Path, unsupported};

/// Where each parameter sits in the parameter blob, and how long each blob is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// The bytes of each parameter in the parameter blob, in declaration order.
    pub params: Vec<Range<usize>>,
    /// The length of the parameter blob.
    pub params_len: usize,
    /// The length of the return blob, or `None` when there is no return value.
    pub ret_len: Option<usize>,
}

/// Checks that every parameter and the return value of `abi` have a byte encoding.
///
/// Refused: a `Field` at any depth, an integer width outside `2..=MAX_INTEGER_WIDTH`, an empty
/// array, string, tuple or struct at any depth, two parameters or two fields of one struct with
/// the same name, a parameter named `return`, and parameters together, or a return value, of
/// [`MAX_INPUT_BYTES`] bytes or more. The error types are not checked: their payloads are never
/// byte-encoded.
pub fn check_abi(abi: &Abi) -> Result<(), Error> {
    layout(abi).map(drop)
}

/// The number of bytes in the encoding of a value of `typ`, which is below [`MAX_INPUT_BYTES`].
/// Refuses the types [`check_abi`] refuses.
pub fn byte_size(typ: &AbiType) -> Result<usize, Error> {
    let path = Path::Root("");
    to_usize(size(typ, path)?, path)
}

/// Where each parameter of `abi` sits in the parameter blob, and how long each blob is. Refuses
/// the ABIs [`check_abi`] refuses.
pub fn layout(abi: &Abi) -> Result<Layout, Error> {
    let mut names = HashSet::new();
    let mut params = Vec::with_capacity(abi.parameters.len());
    let mut end: u64 = 0;
    for param in &abi.parameters {
        let path = Path::Root(&param.name);
        if param.name == MAIN_RETURN_NAME {
            return Err(Error::ReservedName { path: path.to_string() });
        }
        if !names.insert(param.name.as_str()) {
            return Err(Error::DuplicateName { path: path.to_string() });
        }
        let start = end;
        end = capped(start.checked_add(size(&param.typ, path)?), path)?;
        params.push(to_usize(start, path)?..to_usize(end, path)?);
    }
    let params_len = to_usize(end, Path::Root(""))?;

    let ret_len = match &abi.return_type {
        Some(ret) => {
            let path = Path::Root(MAIN_RETURN_NAME);
            Some(to_usize(size(&ret.abi_type, path)?, path)?)
        }
        None => None,
    };
    Ok(Layout { params, params_len, ret_len })
}

/// The number of bytes in the encoding of a value of `typ`, refusing a type with no encoding
/// and a size of [`MAX_INPUT_BYTES`] or more.
fn size(typ: &AbiType, path: Path<'_>) -> Result<u64, Error> {
    let size = match typ {
        AbiType::Field => Err(unsupported(typ, path)),
        AbiType::Integer { width, .. } => {
            if (2..=MAX_INTEGER_WIDTH).contains(width) {
                Ok(u64::from(width.div_ceil(8)))
            } else {
                Err(unsupported(typ, path))
            }
        }
        AbiType::Boolean => Ok(1),
        AbiType::String { length } => Ok(u64::from(*length)),
        AbiType::Array { length, typ } => {
            let element = size(typ, Path::Each(&path))?;
            capped(element.checked_mul(u64::from(*length)), path)
        }
        AbiType::Tuple { fields } => {
            let mut total: u64 = 0;
            for (index, field) in fields.iter().enumerate() {
                let field = size(field, Path::Position(&path, index))?;
                total = capped(total.checked_add(field), path)?;
            }
            Ok(total)
        }
        AbiType::Struct { fields, .. } => {
            let mut names = HashSet::new();
            let mut total: u64 = 0;
            for (name, field) in fields {
                let field_path = Path::Field(&path, name);
                if !names.insert(name.as_str()) {
                    return Err(Error::DuplicateName { path: field_path.to_string() });
                }
                total = capped(total.checked_add(size(field, field_path)?), path)?;
            }
            Ok(total)
        }
    }?;
    // Every element takes a byte or more, so a type of no bytes is an empty array, string,
    // tuple or struct, which no entry point takes or returns.
    if size == 0 {
        return Err(unsupported(typ, path));
    }
    Ok(size)
}

/// A size computed with checked arithmetic, refused when it overflowed or reaches
/// [`MAX_INPUT_BYTES`].
fn capped(size: Option<u64>, path: Path<'_>) -> Result<u64, Error> {
    size.filter(|size| *size < MAX_INPUT_BYTES)
        .ok_or_else(|| Error::TooLarge { path: path.to_string() })
}

/// A size below [`MAX_INPUT_BYTES`] as a `usize`, which holds it on every 32- and 64-bit target.
fn to_usize(size: u64, path: Path<'_>) -> Result<usize, Error> {
    usize::try_from(size).map_err(|_| Error::TooLarge { path: path.to_string() })
}
