use std::collections::BTreeMap;

use acvm::FieldConfig;
use acvm::acir::circuit::ErrorSelector;
use iter_extended::vecmap;
use noirc_abi::{
    ABI_VERSION, Abi, AbiErrorType, AbiParameter, AbiReturnType, AbiType, AbiValue, AbiVisibility,
    Sign,
};
use noirc_errors::Location;
use noirc_evaluator::ErrorType;
use noirc_frontend::TypeBinding;
use noirc_frontend::hir::comptime::{Integer, Value};
use noirc_frontend::shared::{Signedness, Visibility};
use noirc_frontend::{
    hir::{CheckedContext, Context},
    hir_def::{function::Param, stmt::HirPattern, types::Type},
    node_interner::{FuncId, NodeInterner},
};

/// Arranges a function signature and a generated circuit's return witnesses into a
/// `noirc_abi::Abi`.
pub fn gen_abi(
    context: &CheckedContext,
    func_id: &FuncId,
    return_visibility: Visibility,
    error_types: BTreeMap<ErrorSelector, ErrorType>,
) -> Abi {
    let (parameters, return_type) = compute_function_abi(context, func_id);
    assemble_abi(
        context,
        get_main_function_location(context),
        parameters,
        return_type,
        return_visibility,
        error_types,
    )
}

/// Builds an `Abi` from a function's already computed parameters and return type, converting the
/// circuit's error types.
///
/// Converting the error types reads only the crate graph and def maps from `context`, never its
/// `NodeInterner`, so this can run after the interner has been freed. `main_location` is the
/// location reported if a type in the ABI fails to evaluate.
pub(super) fn assemble_abi(
    context: &Context,
    main_location: Location,
    parameters: Vec<AbiParameter>,
    return_type: Option<AbiType>,
    return_visibility: Visibility,
    error_types: BTreeMap<ErrorSelector, ErrorType>,
) -> Abi {
    let return_type = return_type.map(|typ| AbiReturnType {
        abi_type: typ,
        visibility: to_abi_visibility(return_visibility),
    });
    let error_types = error_types
        .into_iter()
        .map(|(selector, typ)| (selector, build_abi_error_type(context, typ, main_location)))
        .collect();
    Abi { abi_version: ABI_VERSION, parameters, return_type, error_types }
}

// Get the Span of the root crate's main function, or else a dummy span if that fails
pub(super) fn get_main_function_location(context: &CheckedContext) -> Location {
    if let Some(func_id) = context.get_main_function(context.root_crate_id()) {
        context.function_meta(&func_id).location
    } else {
        Location::dummy()
    }
}

fn build_abi_error_type(
    context: &Context,
    typ: ErrorType,
    main_location: Location,
) -> AbiErrorType {
    match typ {
        ErrorType::Dynamic(typ) => {
            if let Type::FmtString(len, item_types) = typ {
                let length =
                    len.evaluate_to_u32(main_location).expect("Cannot evaluate fmt length");
                let item_types = match item_types.as_ref() {
                    Type::Tuple(item_types) => {
                        vecmap(item_types, |typ| abi_type_at(context, typ, main_location, false))
                    }
                    Type::Unit => Vec::new(),
                    _ => unreachable!("FmtString items must be a tuple or unit"),
                };
                AbiErrorType::FmtString { length, item_types }
            } else {
                AbiErrorType::Custom(abi_type_at(context, &typ, main_location, false))
            }
        }
        ErrorType::String(string) => AbiErrorType::String { string },
    }
}

/// The ABI type of `typ`. In the signature of an entry point (`main` or a contract function) every
/// type is already known: the elaborator refuses an unbound type there. Elsewhere, as in an error
/// payload, an integer literal whose type was never pinned down reads as the default.
pub(super) fn abi_type_from_hir_type(
    context: &CheckedContext,
    typ: &Type,
    entry_point: bool,
) -> AbiType {
    abi_type_at(context, typ, get_main_function_location(context), entry_point)
}

/// Converts `typ` into an `AbiType`, reporting `main_location` if a length fails to evaluate.
/// See [`abi_type_from_hir_type`] for `entry_point`.
///
/// Reads only the crate graph and def maps from `context`, not its `NodeInterner`.
fn abi_type_at(
    context: &Context,
    typ: &Type,
    main_location: Location,
    entry_point: bool,
) -> AbiType {
    match typ {
        Type::FieldElement => AbiType::Field,
        Type::Array(typ, size) => {
            let length = size
                .evaluate_to_u32(main_location)
                .expect("Cannot have variable sized arrays as a parameter to main");
            let typ = typ.as_ref();
            AbiType::Array {
                length,
                typ: Box::new(abi_type_at(context, typ, main_location, entry_point)),
            }
        }
        Type::Integer(sign, width) => {
            let sign = match sign {
                Signedness::Unsigned => Sign::Unsigned,
                Signedness::Signed => Sign::Signed,
            };
            let width = width
                .evaluate_to_u32(main_location)
                .expect("Cannot have an integer of generic width as a parameter to main");

            AbiType::Integer { sign, width }
        }
        Type::TypeVariable(binding) => {
            if binding.is_integer() || binding.is_integer_or_field() {
                match binding.binding() {
                    TypeBinding::Bound(typ) => {
                        abi_type_at(context, typ, main_location, entry_point)
                    }
                    TypeBinding::Unbound(..) if entry_point => {
                        unreachable!("an entry point's types are bound before its ABI is built")
                    }
                    TypeBinding::Unbound(_id, _kind) => abi_type_at(
                        context,
                        &Type::default_int_or_field_type(),
                        main_location,
                        entry_point,
                    ),
                }
            } else {
                unreachable!("{typ} cannot be used in the abi")
            }
        }
        Type::Bool => AbiType::Boolean,
        Type::String(size) => {
            let size = size
                .evaluate_to_u32(main_location)
                .expect("Cannot have variable sized strings as a parameter to main");
            AbiType::String { length: size }
        }

        Type::DataType(def, args) => {
            let struct_type = def.borrow();
            let fields = struct_type.get_fields(args).unwrap_or_default();
            let fields = vecmap(fields, |(name, typ, _)| {
                (name, abi_type_at(context, &typ, main_location, entry_point))
            });
            // For the ABI, we always want to resolve the struct paths from the root crate
            let path = context.fully_qualified_struct_path(context.root_crate_id(), struct_type.id);
            AbiType::Struct { fields, path }
        }
        Type::Alias(def, args) => {
            abi_type_at(context, &def.borrow().get_type(args), main_location, entry_point)
        }
        Type::CheckedCast { to, .. } => abi_type_at(context, to, main_location, entry_point),
        Type::Tuple(fields) => {
            let fields =
                vecmap(fields, |typ| abi_type_at(context, typ, main_location, entry_point));
            AbiType::Tuple { fields }
        }
        Type::Error
        | Type::Unit
        | Type::Constant(..)
        | Type::InfixExpr(..)
        | Type::TraitAsType(..)
        | Type::NamedGeneric(..)
        | Type::Forall(..)
        | Type::Quoted(_)
        | Type::Vector(_)
        | Type::Function(_, _, _, _) => unreachable!("{typ} cannot be used in the abi"),
        Type::FmtString(_, _) => unreachable!("format strings cannot be used in the abi"),
        Type::Reference(..) => unreachable!("references cannot be used in the abi"),
    }
}

fn to_abi_visibility(value: Visibility) -> AbiVisibility {
    match value {
        Visibility::Public => AbiVisibility::Public,
        Visibility::Private => AbiVisibility::Private,
        Visibility::CallData(_) | Visibility::ReturnData => AbiVisibility::DataBus,
    }
}

pub(super) fn compute_function_abi(
    context: &CheckedContext,
    func_id: &FuncId,
) -> (Vec<AbiParameter>, Option<AbiType>) {
    let func_meta = context.def_interner.function_meta(func_id);

    let parameters = func_meta.parameters.0.clone();
    let return_type = match func_meta.return_type() {
        typ if matches!(typ.follow_bindings(), Type::Unit) => None,
        other => Some(other),
    };

    // Error payloads stay lenient (`build_abi_error_type`): they may hold an unpinned literal.
    let entry_point = func_meta.is_entry_point;
    let parameters = into_abi_params(context, parameters, entry_point);
    let return_type = return_type.map(|typ| abi_type_from_hir_type(context, typ, entry_point));
    (parameters, return_type)
}

/// Retrieves the parameter's bound name. Entry point parameters must use a simple identifier
/// pattern (gated in the elaborator), so any other shape is a compiler bug here.
fn get_param_name<'a>(pattern: &HirPattern, interner: &'a NodeInterner) -> &'a str {
    match pattern {
        HirPattern::Identifier(ident) => interner.definition_name(ident.id),
        HirPattern::Mutable(pattern, _) => get_param_name(pattern, interner),
        HirPattern::Tuple(..) | HirPattern::Struct(..) => {
            unreachable!("destructuring patterns are rejected for entry point parameters")
        }
    }
}

fn into_abi_params(
    context: &CheckedContext,
    params: Vec<Param>,
    entry_point: bool,
) -> Vec<AbiParameter> {
    vecmap(params, |(pattern, typ, vis)| {
        let param_name = get_param_name(&pattern, &context.def_interner).to_owned();
        let as_abi = abi_type_from_hir_type(context, &typ, entry_point);
        AbiParameter { name: param_name, typ: as_abi, visibility: to_abi_visibility(vis) }
    })
}

/// Converts a comptime-evaluated [Value] of an `#[abi(tag)]` global into an [`AbiValue`].
///
/// The elaborator rejects globals whose type the ABI cannot represent (via
/// `Type::program_validity`), so this function only needs to handle the value shapes that
/// correspond to ABI-compatible types.
pub(super) fn value_to_abi_value(value: &Value, field: FieldConfig) -> AbiValue {
    match value {
        Value::Bool(value) => AbiValue::Boolean { value: *value },
        Value::Integer(integer) => {
            let sign = integer.is_negative();
            // Preserve wide integer magnitudes; ABI tags must not reduce them modulo the field.
            let value = match integer {
                Integer::Field(value) => value.to_hex(),
                Integer::Int { value, .. } => {
                    let width = field.num_bytes() as usize * 2;
                    format!("{:0width$x}", value.magnitude())
                }
            };
            AbiValue::Integer { sign, value }
        }
        Value::String(bytes) => match std::str::from_utf8(bytes.as_ref()) {
            Ok(value) => AbiValue::String { value: value.to_string() },
            Err(_) => {
                let value = vecmap(bytes.iter(), |byte| AbiValue::Integer {
                    sign: false,
                    value: format!("{byte:x}"),
                });
                AbiValue::Array { value }
            }
        },
        Value::Array(elements, _) => {
            let value = elements.iter().map(|element| value_to_abi_value(element, field)).collect();
            AbiValue::Array { value }
        }
        Value::Tuple(elements) => {
            let fields = elements
                .iter()
                .map(|element| value_to_abi_value(&element.borrow(), field))
                .collect();
            AbiValue::Tuple { fields }
        }
        Value::Struct(fields, typ) => {
            let Type::DataType(definition, _) = typ.follow_bindings() else {
                unreachable!("Value::Struct must have a DataType type, got: {typ}");
            };
            let definition = definition.borrow();
            let fields_raw =
                definition.fields_raw().expect("ABI-valid struct values must have named fields");
            let abi_fields = vecmap(fields_raw, |struct_field| {
                let name = struct_field.name.as_string();
                let value = fields
                    .iter()
                    .find_map(|(key, value)| (key.as_ref() == name).then_some(value))
                    .expect("ABI struct value should have all fields populated");
                (name.clone(), value_to_abi_value(&value.borrow(), field))
            });
            AbiValue::Struct { fields: abi_fields }
        }
        _ => unreachable!("Value cannot be used in the abi: {value:?}"),
    }
}

#[cfg(test)]
mod tests {
    use acvm::{FieldId, FieldValue};

    use super::*;

    #[test]
    fn abi_tag_integers_preserve_sign_width_and_magnitude() {
        for id in FieldId::ALL {
            let field = FieldConfig::new(id);
            let width = field.num_bytes() as usize * 2;
            for (integer, sign, magnitude) in [
                (
                    Integer::Field(-FieldValue::one(id)),
                    false,
                    (field.modulus() - 1u8).to_str_radix(16),
                ),
                (Integer::i8(i8::MIN), true, "80".to_owned()),
                (Integer::u128(u128::MAX), false, "f".repeat(32)),
            ] {
                assert_eq!(
                    value_to_abi_value(&Value::Integer(integer), field),
                    AbiValue::Integer { sign, value: format!("{magnitude:0>width$}") },
                    "{id}"
                );
            }
        }
    }
}
