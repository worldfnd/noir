use std::rc::Rc;

use iter_extended::vecmap;
use noirc_errors::Location;
use num_bigint::BigInt;

use crate::{
    Type,
    monomorphization::{
        FunctionKey, Monomorphizer,
        ast::{self, Definition, FuncId, Function, InlineType},
        errors::MonomorphizationError,
    },
    node_interner::ExprId,
    shared::{Builtin, Signedness, Visibility},
    token::FmtStrFragment,
};

/// These are the opcodes which the monomorphizer can replace with static results.
/// Any opcode not included is forwarded to SSA as a built-in function.
enum HandledOpcode {
    CheckedTransmute,
    FieldCast,
    ModulusBeBits,
    ModulusBeBytes,
    ModulusLeBits,
    ModulusLeBytes,
    ModulusNumBits,
    // Unlike the others this is a #[foreign] function.
    Poseidon2ConfigStateSize,
    Zeroed,
}

impl Monomorphizer<'_> {
    /// Resolve a `#[builtin]`/`#[foreign]` attribute name to its [`Builtin`], erroring
    /// on names the compiler does not implement (previously an unknown name survived
    /// to SSA generation and panicked there).
    pub(super) fn lookup_builtin(
        name: &str,
        location: Location,
    ) -> Result<Builtin, MonomorphizationError> {
        Builtin::lookup(name).ok_or_else(|| MonomorphizationError::UnknownBuiltin {
            name: name.to_string(),
            location,
        })
    }

    /// Try to evaluate certain builtin functions (just the function itself) given their type.
    /// All builtins are function types, so the evaluated result will always be a new function or None.
    ///
    /// `key` is the function instance being evaluated; the new function is recorded under it.
    pub(super) fn try_evaluate_builtin(
        &mut self,
        builtin: Builtin,
        key: FunctionKey,
        location: Location,
    ) -> Result<Option<FuncId>, MonomorphizationError> {
        let Some(opcode) = HandledOpcode::from_builtin(builtin) else { return Ok(None) };

        let Type::Function(parameter_types, return_type, _env, _unconstrained) = &key.typ else {
            unreachable!("Expected built-in to be a function, found {:?}", key.typ);
        };

        let converted_return_type = Self::convert_type(return_type.as_ref(), location)?;

        let mut parameters = Vec::new();
        let body = match opcode {
            HandledOpcode::CheckedTransmute => {
                assert_eq!(parameter_types.len(), 1);
                let input;
                (parameters, input) = self.builtin_parameter(&parameter_types[0], location)?;

                self.check_transmute(&parameter_types[0], return_type.as_ref(), location)?;

                input
            }
            HandledOpcode::FieldCast => {
                assert_eq!(parameter_types.len(), 1);
                let input;
                (parameters, input) = self.builtin_parameter(&parameter_types[0], location)?;
                Self::field_cast(input, &parameters[0].3, &converted_return_type, location)?
            }
            HandledOpcode::ModulusBeBits => self.modulus_be_bits(location),
            HandledOpcode::ModulusBeBytes => self.modulus_be_bytes(location),
            HandledOpcode::ModulusLeBits => self.modulus_le_bits(location),
            HandledOpcode::ModulusLeBytes => self.modulus_le_bytes(location),
            HandledOpcode::ModulusNumBits => self.modulus_num_bits(location),
            HandledOpcode::Poseidon2ConfigStateSize => Self::poseidon2_config_state_size(location),
            HandledOpcode::Zeroed => self.zeroed_value_of_type(&converted_return_type, location),
        };

        let new_function_id = self.next_function_id();
        self.push_function(
            new_function_id,
            Function {
                id: new_function_id,
                name: self.interner.function_name(&key.id).to_string(),
                parameters,
                body,
                return_type: converted_return_type,
                return_visibility: Visibility::Private,
                unconstrained: key.is_unconstrained,
                inline_type: InlineType::InlineAlways,
                is_entry_point: false,
                allow_constant_return: false,
            },
        );
        self.define_function(key, new_function_id);
        Ok(Some(new_function_id))
    }

    /// The parameter `x` of a one-parameter builtin evaluated to a function, and an expression
    /// that reads it.
    fn builtin_parameter(
        &mut self,
        typ: &Type,
        location: Location,
    ) -> Result<(ast::Parameters, ast::Expression), MonomorphizationError> {
        let id = self.next_local_id();
        let typ = Rc::new(Self::convert_type(typ, location)?);
        let name = "x".to_string();
        let input = ast::Expression::Ident(ast::Ident {
            location: Some(location),
            definition: Definition::Local(id),
            mutable: false,
            name: name.clone(),
            typ: typ.clone(),
            id: self.next_ident_id(),
        });
        Ok((vec![(id, false, name, typ, Visibility::Private)], input))
    }

    /// The cast between `Field` and an integer that a `field_cast` call stands for. A signed
    /// integer enters the field as its bit pattern, through the unsigned type of its width.
    fn field_cast(
        input: ast::Expression,
        from: &ast::Type,
        to: &ast::Type,
        location: Location,
    ) -> Result<ast::Expression, MonomorphizationError> {
        let cast =
            |lhs, r#type| ast::Expression::Cast(ast::Cast { lhs: Box::new(lhs), r#type, location });
        match (from, to) {
            (ast::Type::Integer(Signedness::Signed, bits), ast::Type::Field) => {
                let pattern = cast(input, ast::Type::Integer(Signedness::Unsigned, *bits));
                Ok(cast(pattern, ast::Type::Field))
            }
            (ast::Type::Integer(..), ast::Type::Field)
            | (ast::Type::Field, ast::Type::Integer(..)) => Ok(cast(input, to.clone())),
            _ => {
                let message = "ICE: `field_cast` converts between `Field` and an integer";
                Err(MonomorphizationError::InternalError { location, message })
            }
        }
    }

    /// Check the given type conversion that the types are equal, or issue an error if not.
    fn check_transmute(
        &self,
        actual: &Type,
        expected: &Type,
        location: Location,
    ) -> Result<(), MonomorphizationError> {
        if actual.try_unify_with_default_bindings(expected).is_ok() {
            Ok(())
        } else {
            let actual = actual.to_string();
            let expected = expected.to_string();
            Err(MonomorphizationError::CheckedTransmuteFailed { actual, expected, location })
        }
    }

    fn modulus_bool_vector_literal(&self, bits: Vec<u8>, _location: Location) -> ast::Expression {
        use ast::*;

        let bits_as_expr = vecmap(bits, |bit| Expression::Literal(Literal::Bool(bit != 0)));

        let typ = Type::Vector(Rc::new(Type::Bool));
        let arr_literal = ArrayLiteral { typ, contents: bits_as_expr };
        Expression::Literal(Literal::Vector(arr_literal))
    }

    fn modulus_vector_literal(
        &self,
        bytes: Vec<u8>,
        arr_elem_bits: u32,
        location: Location,
    ) -> ast::Expression {
        use ast::*;

        let int_type = Type::Integer(Signedness::Unsigned, arr_elem_bits);
        assert!(
            arr_elem_bits >= 8,
            "modulus_vector_literal: arr_elem_bits ({arr_elem_bits}) is too small to hold a u8 byte",
        );

        let bytes_as_expr = vecmap(bytes, |byte| {
            Expression::Literal(Literal::Integer(byte.into(), int_type.clone(), location))
        });

        let typ = Type::Vector(Rc::new(int_type));
        let arr_literal = ArrayLiteral { typ, contents: bytes_as_expr };
        Expression::Literal(Literal::Vector(arr_literal))
    }

    /// Implements `std::mem::zeroed` by returning an appropriate zeroed
    /// ast literal or collection node for the given type. Note that for functions
    /// there is no obvious zeroed value so this should be considered unsafe to use.
    pub(super) fn zeroed_value_of_type(
        &mut self,
        typ: &ast::Type,
        location: Location,
    ) -> ast::Expression {
        match typ {
            ast::Type::Field | ast::Type::Integer(..) => {
                let typ = typ.clone();
                let zero = BigInt::ZERO;
                ast::Expression::Literal(ast::Literal::Integer(zero, typ, location))
            }
            ast::Type::Bool => ast::Expression::Literal(ast::Literal::Bool(false)),
            ast::Type::Unit => ast::Expression::Literal(ast::Literal::Unit),
            ast::Type::Array(length, element_type) => {
                let typ = ast::Type::Array(*length, element_type.clone());
                if element_type.contains_reference() {
                    // A reference lowers to a single `allocate`, so a repeated array literal would
                    // make every slot share one allocation. Author N independent elements instead
                    // so each slot gets its own cell, matching the `Tuple` arm below.
                    let contents =
                        vecmap(0..*length, |_| self.zeroed_value_of_type(element_type, location));
                    ast::Expression::Literal(ast::Literal::Array(ast::ArrayLiteral {
                        contents,
                        typ,
                    }))
                } else {
                    let element = self.zeroed_value_of_type(element_type, location);
                    ast::Expression::Literal(ast::Literal::Repeated {
                        element: Box::new(element),
                        length: *length,
                        is_vector: false,
                        typ,
                    })
                }
            }
            ast::Type::String(length) => {
                ast::Expression::Literal(ast::Literal::Str(vec![0; *length as usize]))
            }
            ast::Type::FmtString(length, fields) => {
                let zeroed_tuple = self.zeroed_value_of_type(fields, location);
                let fields_len = match &zeroed_tuple {
                    ast::Expression::Tuple(fields) => fields.len() as u64,
                    _ => unreachable!(
                        "ICE: format string fields should be structured in a tuple, but got a {zeroed_tuple}"
                    ),
                };
                ast::Expression::Literal(ast::Literal::FmtStr(
                    vec![FmtStrFragment::String("\0".repeat(*length as usize))],
                    fields_len,
                    Box::new(zeroed_tuple),
                ))
            }
            ast::Type::Tuple(fields) => ast::Expression::Tuple(vecmap(fields, |field| {
                self.zeroed_value_of_type(field, location)
            })),
            ast::Type::Function(parameter_types, ret_type, env, unconstrained) => self
                .create_zeroed_function(
                    parameter_types,
                    ret_type.clone(),
                    env.clone(),
                    *unconstrained,
                    location,
                ),
            ast::Type::Vector(element_type) => {
                ast::Expression::Literal(ast::Literal::Vector(ast::ArrayLiteral {
                    contents: vec![],
                    typ: ast::Type::Vector(element_type.clone()),
                }))
            }
            ast::Type::Reference(element, mutable) => {
                let rhs = Box::new(self.zeroed_value_of_type(element, location));
                let result_type = typ.clone();
                ast::Expression::Unary(ast::Unary {
                    rhs,
                    result_type,
                    operator: super::UnaryOp::Reference { mutable: *mutable },
                    location,
                    skip: false,
                })
            }
        }
    }

    // Creating a zeroed function value is almost always an error if it is used later,
    // Hence why std::unsafe_func::zeroed is unsafe.
    //
    // To avoid confusing later passes, we arbitrarily choose to construct a function
    // that satisfies the input type by discarding all its parameters and returning a
    // zeroed value of the result type.
    fn create_zeroed_function(
        &mut self,
        parameter_types: &[ast::Type],
        ret_type: Rc<ast::Type>,
        env_type: Rc<ast::Type>,
        unconstrained: bool,
        location: Location,
    ) -> ast::Expression {
        let lambda_name = "zeroed_lambda";

        let mut parameters = Vec::with_capacity(parameter_types.len() + 1);
        if !matches!(env_type.as_ref(), ast::Type::Unit) {
            parameters.push((
                self.next_local_id(),
                true,
                "env".into(),
                env_type.clone(),
                Visibility::Private,
            ));
        }
        parameters.extend(parameter_types.iter().map(|parameter_type| {
            (
                self.next_local_id(),
                false,
                "_".into(),
                Rc::new(parameter_type.clone()),
                Visibility::Private,
            )
        }));

        let body = self.zeroed_value_of_type(&ret_type, location);

        let id = self.next_function_id();
        let return_type = ret_type.clone();
        let name = lambda_name.to_owned();

        let function = Function {
            id,
            name,
            parameters,
            body,
            return_type: return_type.as_ref().clone(),
            return_visibility: Visibility::Private,
            unconstrained,
            inline_type: InlineType::default(),
            is_entry_point: false,
            allow_constant_return: false,
        };
        self.push_function(id, function);

        ast::Expression::Ident(ast::Ident {
            definition: Definition::Function(id),
            mutable: false,
            location: None,
            name: lambda_name.to_owned(),
            typ: Rc::new(ast::Type::Function(
                parameter_types.to_owned(),
                ret_type,
                env_type,
                unconstrained,
            )),
            id: self.next_ident_id(),
        })
    }

    /// Try to call certain builtin functions with the given arguments,
    /// returning the result as an expression.
    pub(super) fn try_evaluate_builtin_call(
        &mut self,
        func: &ast::Expression,
        expr_id: &ExprId,
        arguments: &[ExprId],
        argument_values: &[ast::Expression],
        result_type: &ast::Type,
    ) -> Result<Option<ast::Expression>, MonomorphizationError> {
        if let ast::Expression::Ident(ident) = func
            && let Definition::Builtin(opcode) | Definition::LowLevel(opcode) = &ident.definition
        {
            let location = self.interner.expr_location(expr_id);

            return Ok(Some(match HandledOpcode::from_builtin(*opcode) {
                Some(HandledOpcode::CheckedTransmute) => {
                    assert_eq!(arguments.len(), 1);
                    let parameter_type = self.expr_type(arguments[0]).follow_bindings();
                    let result_type = self.expr_type(*expr_id).follow_bindings();
                    self.check_transmute(&parameter_type, &result_type, location)?;
                    argument_values[0].clone()
                }
                Some(HandledOpcode::FieldCast) => {
                    assert_eq!(arguments.len(), 1);
                    let parameter_type = self.expr_type(arguments[0]).follow_bindings();
                    let parameter_type = Self::convert_type(&parameter_type, location)?;
                    let input = argument_values[0].clone();
                    Self::field_cast(input, &parameter_type, result_type, location)?
                }
                Some(HandledOpcode::ModulusBeBits) => self.modulus_be_bits(location),
                Some(HandledOpcode::ModulusBeBytes) => self.modulus_be_bytes(location),
                Some(HandledOpcode::ModulusLeBits) => self.modulus_le_bits(location),
                Some(HandledOpcode::ModulusLeBytes) => self.modulus_le_bytes(location),
                Some(HandledOpcode::ModulusNumBits) => self.modulus_num_bits(location),
                Some(HandledOpcode::Poseidon2ConfigStateSize) => {
                    Self::poseidon2_config_state_size(location)
                }
                Some(HandledOpcode::Zeroed) => self.zeroed_value_of_type(result_type, location),
                None => return Ok(None),
            }));
        }
        Ok(None)
    }

    fn modulus_be_bits(&self, location: Location) -> ast::Expression {
        let bits = self.interner.field().modulus().to_radix_be(2);
        self.modulus_bool_vector_literal(bits, location)
    }

    fn modulus_be_bytes(&self, location: Location) -> ast::Expression {
        let bytes = self.interner.field().modulus().to_bytes_be();
        self.modulus_vector_literal(bytes, 8, location)
    }

    fn modulus_le_bits(&self, location: Location) -> ast::Expression {
        let bits = self.interner.field().modulus().to_radix_le(2);
        self.modulus_bool_vector_literal(bits, location)
    }

    fn modulus_le_bytes(&self, location: Location) -> ast::Expression {
        let bytes = self.interner.field().modulus().to_bytes_le();
        self.modulus_vector_literal(bytes, 8, location)
    }

    fn modulus_num_bits(&self, location: Location) -> ast::Expression {
        let bits = self.interner.field().num_bits();
        let typ = ast::Type::Integer(Signedness::Unsigned, 64);
        ast::Expression::Literal(ast::Literal::Integer(bits.into(), typ, location))
    }

    fn poseidon2_config_state_size(location: Location) -> ast::Expression {
        let size = bn254_blackbox_solver::poseidon2_config_state_size();
        let typ = ast::Type::Integer(Signedness::Unsigned, 32);
        ast::Expression::Literal(ast::Literal::Integer(size.into(), typ, location))
    }
}

impl HandledOpcode {
    fn from_builtin(builtin: Builtin) -> Option<Self> {
        match builtin {
            Builtin::CheckedTransmute => Some(Self::CheckedTransmute),
            Builtin::FieldCast => Some(Self::FieldCast),
            Builtin::ModulusBeBits => Some(Self::ModulusBeBits),
            Builtin::ModulusBeBytes => Some(Self::ModulusBeBytes),
            Builtin::ModulusLeBits => Some(Self::ModulusLeBits),
            Builtin::ModulusLeBytes => Some(Self::ModulusLeBytes),
            Builtin::ModulusNumBits => Some(Self::ModulusNumBits),
            Builtin::Poseidon2ConfigStateSize => Some(Self::Poseidon2ConfigStateSize),
            Builtin::Zeroed => Some(Self::Zeroed),
            _ => None,
        }
    }
}
