use noirc_errors::{CustomDiagnostic, Location};

use crate::{NamedGeneric, Type, TypeBinding, ast::Ident, recursion::TypeRecursionContext};

/// Where a type crosses into a program, which decides the rules it follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProgramBoundary {
    /// `main` or a contract function. Its inputs and return cross as bytes, the same on every
    /// field, so they carry no `Field` and no type that is unbound or field-specific.
    EntryPoint,
    /// A test or fuzz function, the return of a `#[no_predicates]` function, or an `#[abi(tag)]`
    /// global: any sized type, `Field` included.
    Harness,
}

/// An type incorrectly used as a program input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidType {
    Primitive(Type),
    Enum(Type),
    EmptyArray(Type),
    EmptyString(Type),
    Alias {
        alias_name: Ident,
        invalid_type: Box<InvalidType>,
    },
    StructField {
        struct_name: Ident,
        field_name: Ident,
        invalid_type: Box<InvalidType>,
    },
    /// `Field`, whose values depend on the field the program is compiled for.
    Field(Type),
    /// A type that is still unbound, so it has no size at the boundary, as it was spelled when it
    /// was checked: inference may bind it later, and the diagnostic must not show that binding.
    Unbound(String),
    /// A struct, enum or alias marked `#[field(..)]`, itself or through an enclosing module.
    FieldGated {
        typ: Type,
        gate: Location,
    },
}

impl InvalidType {
    /// The invalid type itself, past the aliases and struct fields that contain it.
    pub(crate) fn innermost(&self) -> &InvalidType {
        match self {
            InvalidType::Alias { invalid_type, .. }
            | InvalidType::StructField { invalid_type, .. } => invalid_type.innermost(),
            other => other,
        }
    }

    pub(crate) fn add_to_diagnostic(&self, location: Location, diagnostic: &mut CustomDiagnostic) {
        self.add_to_diagnostic_in_context(location, diagnostic, "entry point");
    }

    pub(crate) fn add_to_abi_diagnostic(
        &self,
        location: Location,
        diagnostic: &mut CustomDiagnostic,
    ) {
        self.add_to_diagnostic_in_context(location, diagnostic, "ABI");
    }

    fn add_to_diagnostic_in_context(
        &self,
        location: Location,
        diagnostic: &mut CustomDiagnostic,
        context: &str,
    ) {
        match self {
            InvalidType::Primitive(typ) => match typ {
                Type::Unit => {
                    diagnostic
                        .add_secondary(format!("Unit is not a valid {context} type"), location);
                }
                Type::Reference(..) => {
                    diagnostic.add_secondary(
                        format!("Reference is not a valid {context} type. Found: {typ}"),
                        location,
                    );
                }
                Type::Vector(..) => {
                    diagnostic.add_secondary(
                        format!("Vector is not a valid {context} type. Found: {typ}"),
                        location,
                    );
                }
                _ => {
                    diagnostic.add_secondary(format!("Invalid {context} type: {typ}"), location);
                }
            },
            InvalidType::Enum(typ) => {
                diagnostic.add_secondary(
                    format!("Enum is not yet allowed as an {context} type. Found: {typ}"),
                    location,
                );
            }
            InvalidType::EmptyArray(typ) => {
                diagnostic.add_secondary(
                    format!("Empty array is not a valid {context} type. Found: {typ}"),
                    location,
                );
            }
            InvalidType::EmptyString(typ) => {
                diagnostic.add_secondary(
                    format!("Empty string is not a valid {context} type. Found: {typ}"),
                    location,
                );
            }
            InvalidType::Field(typ) => {
                diagnostic.add_secondary(
                    format!("Field is not a valid {context} type. Found: {typ}"),
                    location,
                );
            }
            InvalidType::Unbound(typ) => {
                diagnostic.add_secondary(
                    format!("An unbound type is not a valid {context} type. Found: {typ}"),
                    location,
                );
            }
            InvalidType::FieldGated { typ, gate } => {
                diagnostic.add_secondary(
                    format!("A field-specific type is not a valid {context} type. Found: {typ}"),
                    location,
                );
                diagnostic.add_secondary("Made field-specific here".to_string(), *gate);
            }
            InvalidType::StructField { struct_name, field_name, invalid_type } => {
                diagnostic.add_secondary(
                    format!("Struct {struct_name} has an invalid {context} type"),
                    struct_name.location(),
                );
                diagnostic.add_secondary(
                    format!("Field {field_name} has an invalid {context} type"),
                    field_name.location(),
                );
                invalid_type.add_to_diagnostic_in_context(
                    field_name.location(),
                    diagnostic,
                    context,
                );
            }
            InvalidType::Alias { alias_name, invalid_type } => {
                diagnostic.add_secondary(
                    format!("Alias {alias_name} has an invalid {context} type"),
                    alias_name.location(),
                );
                invalid_type.add_to_diagnostic_in_context(
                    alias_name.location(),
                    diagnostic,
                    context,
                );
            }
        }
    }
}

impl Type {
    /// Returns this type, or a nested one, that cannot cross `boundary`.
    ///
    /// Every boundary refuses unsized types, such as vectors and named generics, types that make
    /// no sense as a program input, such as references, and zero-length arrays and strings as
    /// inputs. An [`ProgramBoundary::EntryPoint`] also refuses `Field`, unbound types and
    /// field-specific types. Integers of any width cross; the backend decides which widths it
    /// lowers.
    ///
    /// This function should match the same check done in `create_value_from_type` in `acir_gen`.
    /// If this function does not catch a case where a type should be valid, it will later lead to a
    /// panic in that function instead of a user-facing compiler error message.
    ///
    /// Returns `None` if this type and its nested types are all valid program inputs.
    pub(crate) fn program_validity(
        &self,
        output: bool,
        boundary: ProgramBoundary,
    ) -> Option<InvalidType> {
        fn helper(
            this: &Type,
            allow_empty_arrays: bool,
            allow_unit: bool,
            boundary: ProgramBoundary,
            mut type_recursion_context: TypeRecursionContext,
        ) -> Option<InvalidType> {
            let entry_point = boundary == ProgramBoundary::EntryPoint;
            match this {
                Type::FieldElement if entry_point => Some(InvalidType::Field(this.clone())),

                // Type::Error is allowed as usual since it indicates an error was already issued and
                // we don't need to issue further errors about this likely unresolved type
                Type::FieldElement | Type::Bool | Type::Constant(_) | Type::Error => None,

                // Unit at the outer return means no return value. Reach it through aliases so
                // an entry point still checks a field gate on the alias itself.
                Type::Unit if allow_unit => None,

                // An integer needs a concrete width, like an array needs a concrete length: a
                // width that still names a generic has no size at the boundary.
                Type::Integer(_, width) => width
                    .evaluate_to_u32(Location::dummy())
                    .is_err()
                    .then(|| InvalidType::Primitive(this.clone())),

                Type::Unit
                | Type::FmtString(_, _)
                | Type::Function(_, _, _, _)
                | Type::Reference(..)
                | Type::Forall(_, _)
                | Type::Quoted(_)
                | Type::Vector(_)
                | Type::TraitAsType(..) => Some(InvalidType::Primitive(this.clone())),

                Type::CheckedCast { to, .. } => helper(
                    to,
                    allow_empty_arrays,
                    allow_unit,
                    boundary,
                    type_recursion_context.recur(),
                ),

                Type::Alias(alias, generics) => {
                    let alias = alias.borrow();
                    if entry_point && let Some(gate) = alias.field_gate {
                        return Some(InvalidType::FieldGated { typ: this.clone(), gate });
                    }
                    if type_recursion_context.insert_alias(alias.id, generics.clone()) {
                        if let Some(invalid_type) = helper(
                            &alias.get_type(generics),
                            allow_empty_arrays,
                            allow_unit,
                            boundary,
                            type_recursion_context.recur(),
                        ) {
                            let alias_name = alias.name.clone();
                            Some(InvalidType::Alias {
                                alias_name,
                                invalid_type: Box::new(invalid_type),
                            })
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }

                Type::Array(element, length) => {
                    if !length_is_valid_for_entry_point(length, allow_empty_arrays) {
                        Some(InvalidType::Primitive(this.clone()))
                    } else {
                        helper(
                            length,
                            allow_empty_arrays,
                            false,
                            boundary,
                            type_recursion_context.clone().recur(),
                        )
                        .or_else(|| {
                            helper(
                                element,
                                allow_empty_arrays,
                                false,
                                boundary,
                                type_recursion_context.recur(),
                            )
                        })
                    }
                }
                Type::String(length) => {
                    if !length_is_valid_for_entry_point(length, allow_empty_arrays) {
                        Some(InvalidType::EmptyString(this.clone()))
                    } else {
                        helper(
                            length,
                            allow_empty_arrays,
                            false,
                            boundary,
                            type_recursion_context.recur(),
                        )
                    }
                }
                Type::Tuple(elements) => {
                    for element in elements {
                        if let Some(invalid_type) = helper(
                            element,
                            allow_empty_arrays,
                            false,
                            boundary,
                            type_recursion_context.clone().recur(),
                        ) {
                            return Some(invalid_type);
                        }
                    }
                    None
                }
                Type::DataType(definition, generics) => {
                    if entry_point && let Some(gate) = definition.borrow().field_gate {
                        return Some(InvalidType::FieldGated { typ: this.clone(), gate });
                    }
                    if type_recursion_context
                        .insert_data_type(definition.borrow().id, generics.clone())
                    {
                        let definition = definition.borrow();

                        if let Some(fields) = definition.get_fields(generics) {
                            for (field_name, field, _) in fields {
                                if let Some(invalid_type) = helper(
                                    &field,
                                    allow_empty_arrays,
                                    false,
                                    boundary,
                                    type_recursion_context.clone().recur(),
                                ) {
                                    let struct_name = definition.name.clone();
                                    let mut fields_raw = definition.fields_raw().unwrap().iter();
                                    let field =
                                        fields_raw.find(|field| field.name.as_str() == field_name);
                                    return Some(InvalidType::StructField {
                                        struct_name,
                                        field_name: field.unwrap().name.clone(),
                                        invalid_type: Box::new(invalid_type),
                                    });
                                }
                            }
                            None
                        } else {
                            // Arbitrarily disallow enums from program input, though we may support them later
                            Some(InvalidType::Enum(this.clone()))
                        }
                    } else {
                        None
                    }
                }
                Type::InfixExpr(lhs, _, rhs, _) => helper(
                    lhs,
                    allow_empty_arrays,
                    false,
                    boundary,
                    type_recursion_context.clone().recur(),
                )
                .or_else(|| {
                    helper(rhs, allow_empty_arrays, false, boundary, type_recursion_context.recur())
                }),
                Type::TypeVariable(type_var)
                | Type::NamedGeneric(NamedGeneric { type_var, .. }) => {
                    if let TypeBinding::Bound(typ) = &*type_var.borrow() {
                        helper(
                            typ,
                            allow_empty_arrays,
                            allow_unit,
                            boundary,
                            type_recursion_context.recur(),
                        )
                    } else if entry_point {
                        // An entry point's types are all known before it runs.
                        Some(InvalidType::Unbound(this.to_string()))
                    } else {
                        // Elsewhere an unbound type can only come from the function's own
                        // generics, which are reported separately.
                        None
                    }
                }
            }
        }

        // Zero-length arrays and strings may be returned, but not taken.
        helper(self, output, output, boundary, TypeRecursionContext::default())
    }

    /// Returns this type, or a nested one, if this type can be used as a parameter to an ACIR
    /// function that is not `main` or a contract function.
    /// This encapsulates functions for which we may not want to inline during compilation.
    ///
    /// This check is intentionally more permissive than [`Self::program_validity`]:
    /// - It does not enforce entry-point sizing rules (e.g. concrete array/string lengths).
    /// - It allows symbolic size expressions such as [`Type::InfixExpr`].
    /// - It does not special-case entry point only rules like allowing [`Type::Unit`] outputs.
    ///
    /// The inputs allowed for a function entry point differ from those allowed as input to a program as there are
    /// certain types which through compilation we know what their size should be.
    /// This includes types such as numeric generics.
    pub(crate) fn non_inlined_function_input_validity(&self) -> Option<InvalidType> {
        self.non_inlined_function_input_validity_helper(TypeRecursionContext::default())
    }

    fn non_inlined_function_input_validity_helper(
        &self,
        mut type_recursion_context: TypeRecursionContext,
    ) -> Option<InvalidType> {
        match self {
            // Type::Error is allowed as usual since it indicates an error was already issued and
            // we don't need to issue further errors about this likely unresolved type
            Type::FieldElement
            | Type::Integer(_, _)
            | Type::Bool
            | Type::Constant(_)
            | Type::InfixExpr(..)
            | Type::Error => None,

            Type::Unit
            | Type::FmtString(_, _)
            // To enable this we would need to determine the size of the closure outputs at compile-time.
            // This is possible as long as the output size is not dependent upon a witness condition.
            | Type::Function(_, _, _, _)
            | Type::Vector(_)
            | Type::Reference(..)
            | Type::Forall(_, _)
            // TODO: probably can allow code as it is all compile time
            | Type::Quoted(_)
            | Type::TraitAsType(..) => Some(InvalidType::Primitive(self.clone())),

            Type::CheckedCast { to, .. } => to.non_inlined_function_input_validity_helper(type_recursion_context.recur()),

            Type::Alias(alias, generics) => {
                if type_recursion_context.insert_alias(alias.borrow().id, generics.clone()) {

                let alias = alias.borrow();
                if let Some(invalid_type) = alias.get_type(generics).non_inlined_function_input_validity_helper(type_recursion_context.recur()) {
                    let alias_name = alias.name.clone();
                    Some(InvalidType::Alias { alias_name, invalid_type: Box::new(invalid_type) })
                } else {
                    None
                }
            } else {
                None

            }
            }

            Type::Array(element, length) => {
                length.non_inlined_function_input_validity_helper(type_recursion_context.clone().recur()).or_else(|| element.non_inlined_function_input_validity_helper(type_recursion_context.recur()))
            }
            Type::String(length) => length.non_inlined_function_input_validity_helper(type_recursion_context.recur()),
            Type::Tuple(elements) => {
                for element in elements {
                    if let Some(invalid_type) = element.non_inlined_function_input_validity_helper(type_recursion_context.clone().recur()) {
                        return Some(invalid_type);
                    }
                }
                None
            },
            Type::DataType(definition, generics) => {
                if type_recursion_context.insert_data_type(definition.borrow().id, generics.clone()) {
                    let definition = definition.borrow();

                    if let Some(fields) = definition.get_fields(generics) {
                        for (field_name, field, _) in fields {
                            if let Some(invalid_type) = field.non_inlined_function_input_validity_helper(type_recursion_context.clone().recur()) {
                                let struct_name = definition.name.clone();
                                let mut fields_raw = definition.fields_raw().unwrap().iter();
                                let field = fields_raw.find(|field| field.name.as_str() == field_name);
                                return Some(InvalidType::StructField {
                                    struct_name,
                                    field_name: field.unwrap().name.clone(),
                                    invalid_type: Box::new(invalid_type),
                                });
                            }
                        }
                        None
                    } else {
                        Some(InvalidType::Enum(self.clone()))
                    }
                } else {
                    None
                }
            }
            Type::TypeVariable(type_var)
            | Type::NamedGeneric(NamedGeneric { type_var, .. }) => {
                // Unbound TypeVariable and Generic are allowed here as they can only result from
                // generics being declared on the function itself, but we produce a different error in that case.
                if let TypeBinding::Bound(typ) = &*type_var.borrow() {
                    typ.non_inlined_function_input_validity_helper(type_recursion_context.recur())
                } else {
                    None
                }
            }
        }
    }

    /// Returns true if a value of this type can safely pass between constrained and
    /// unconstrained functions (and vice-versa).
    pub(crate) fn is_valid_for_unconstrained_boundary(&self) -> bool {
        self.is_valid_for_unconstrained_boundary_helper(TypeRecursionContext::default())
    }

    fn is_valid_for_unconstrained_boundary_helper(
        &self,
        mut type_recursion_context: TypeRecursionContext,
    ) -> bool {
        match self {
            Type::FieldElement
            | Type::Integer(_, _)
            | Type::Bool
            | Type::Unit
            | Type::Constant(_)
            | Type::Vector(_)
            | Type::Function(_, _, _, _)
            | Type::FmtString(_, _)
            | Type::InfixExpr(..)
            | Type::Error => true,

            Type::TypeVariable(type_var) | Type::NamedGeneric(NamedGeneric { type_var, .. }) => {
                if let TypeBinding::Bound(typ) = &*type_var.borrow() {
                    typ.is_valid_for_unconstrained_boundary_helper(type_recursion_context.recur())
                } else {
                    true
                }
            }

            Type::CheckedCast { to, .. } => {
                to.is_valid_for_unconstrained_boundary_helper(type_recursion_context.recur())
            }

            // Quoted objects only exist at compile-time where the only execution
            // environment is the interpreter. In this environment, they are valid.
            Type::Quoted(_) => true,

            // Mutable references cannot cross the constrained/unconstrained boundary.
            // Immutable references are allowed if their inner type is also valid.
            Type::Reference(inner, mutable) => {
                if *mutable {
                    false
                } else {
                    inner.is_valid_for_unconstrained_boundary_helper(type_recursion_context.recur())
                }
            }
            Type::Forall(_, _) | Type::TraitAsType(..) => false,

            Type::Alias(alias, generics) => {
                if type_recursion_context.insert_alias(alias.borrow().id, generics.clone()) {
                    let alias = alias.borrow();
                    alias
                        .get_type(generics)
                        .is_valid_for_unconstrained_boundary_helper(type_recursion_context.recur())
                } else {
                    true
                }
            }

            Type::Array(element, length) => {
                length.is_valid_for_unconstrained_boundary_helper(
                    type_recursion_context.clone().recur(),
                ) && element
                    .is_valid_for_unconstrained_boundary_helper(type_recursion_context.recur())
            }
            Type::String(length) => {
                length.is_valid_for_unconstrained_boundary_helper(type_recursion_context.recur())
            }
            Type::Tuple(elements) => elements.iter().all(|elem| {
                elem.is_valid_for_unconstrained_boundary_helper(
                    type_recursion_context.clone().recur(),
                )
            }),
            Type::DataType(definition, generics) => {
                let definition = definition.borrow();
                if type_recursion_context.insert_data_type(definition.id, generics.clone()) {
                    if let Some(fields) = definition.get_fields(generics) {
                        fields.into_iter().all(|(_, field, _)| {
                            field.is_valid_for_unconstrained_boundary_helper(
                                type_recursion_context.clone().recur(),
                            )
                        })
                    } else if let Some(variants) = definition.get_variants(generics) {
                        // An enum can be passed into an unconstrained function: it was built in
                        // the constrained caller so its tag is already valid. Returning one the
                        // other way is rejected separately (see `unconstrained_function_return`).
                        variants.into_iter().flat_map(|(_, args)| args).all(|typ| {
                            typ.is_valid_for_unconstrained_boundary_helper(
                                type_recursion_context.clone().recur(),
                            )
                        })
                    } else {
                        true
                    }
                } else {
                    true
                }
            }
        }
    }
}

pub(crate) fn length_is_valid_for_entry_point(length: &Type, allow_empty: bool) -> bool {
    match length.evaluate_to_u32(Location::dummy()) {
        Ok(0) => allow_empty, // Zero is invalid unless allow_empty
        Ok(_) => true,        // Positive is always valid
        Err(_) => false,      // Failed to evaluate (like -1) is invalid
    }
}
