//! Primitive type definitions

use iter_extended::vecmap;
use noirc_errors::Location;

use crate::{
    QuotedType, Type,
    ast::{GenericTypeArgs, UnresolvedTypeData, UnresolvedTypeExpression},
    elaborator::{Elaborator, PathResolutionMode, Turbofish, types::WildcardAllowed},
    hir::{
        def_collector::dc_crate::CompilationError,
        resolution::errors::ResolverError,
        type_check::{
            TypeCheckError,
            generics::{FmtstrPrimitiveType, Generic as _, IntegerPrimitiveType, StrPrimitiveType},
        },
    },
    shared::{Signedness, is_legal_integer_width, parse_integer_type_name},
    token::IntegerTypeSuffix,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    Bool,
    CtString,
    Expr,
    Field,
    Fmtstr,
    FunctionDefinition,
    /// A named integer type such as `u8` or `i512`. The width is legal by construction:
    /// [`PrimitiveType::lookup_by_name`] admits only the widths the language has.
    Integer(Signedness, u32),
    Location,
    Module,
    Quoted,
    Str,
    TraitConstraint,
    TraitDefinition,
    TraitImpl,
    TypeDefinition,
    TypedExpr,
    Type,
    UnresolvedType,
}

impl PrimitiveType {
    /// The primitive types the standard library documents and tooling lists by name: every
    /// non-integer primitive and the integer types up to `u128`. Wider integer types and
    /// `i128` resolve by name all the same; they are only absent from this list.
    pub const NAMED: [PrimitiveType; 26] = [
        Self::Bool,
        Self::CtString,
        Self::Expr,
        Self::Field,
        Self::Fmtstr,
        Self::FunctionDefinition,
        Self::Integer(Signedness::Signed, 8),
        Self::Integer(Signedness::Signed, 16),
        Self::Integer(Signedness::Signed, 32),
        Self::Integer(Signedness::Signed, 64),
        Self::Integer(Signedness::Unsigned, 8),
        Self::Integer(Signedness::Unsigned, 16),
        Self::Integer(Signedness::Unsigned, 32),
        Self::Integer(Signedness::Unsigned, 64),
        Self::Integer(Signedness::Unsigned, 128),
        Self::Location,
        Self::Module,
        Self::Quoted,
        Self::Str,
        Self::TraitConstraint,
        Self::TraitDefinition,
        Self::TraitImpl,
        Self::TypeDefinition,
        Self::TypedExpr,
        Self::Type,
        Self::UnresolvedType,
    ];

    pub fn lookup_by_name(name: &str) -> Option<Self> {
        match name {
            "bool" => Some(Self::Bool),
            "CtString" => Some(Self::CtString),
            "Expr" => Some(Self::Expr),
            "fmtstr" => Some(Self::Fmtstr),
            "Field" => Some(Self::Field),
            "FunctionDefinition" => Some(Self::FunctionDefinition),
            "Location" => Some(Self::Location),
            "Module" => Some(Self::Module),
            "Quoted" => Some(Self::Quoted),
            "str" => Some(Self::Str),
            "TraitConstraint" => Some(Self::TraitConstraint),
            "TraitDefinition" => Some(Self::TraitDefinition),
            "TraitImpl" => Some(Self::TraitImpl),
            "TypeDefinition" => Some(Self::TypeDefinition),
            "TypedExpr" => Some(Self::TypedExpr),
            "Type" => Some(Self::Type),
            "UnresolvedType" => Some(Self::UnresolvedType),
            _ => {
                let (signedness, bits) = parse_integer_type_name(name)?;
                is_legal_integer_width(bits).then_some(Self::Integer(signedness, bits))
            }
        }
    }

    /// The parametric integer families, `u<N>` and `i<N>`. The letter alone is an ordinary
    /// identifier, so this is consulted only where generic arguments follow the name.
    pub fn integer_family(name: &str) -> Option<Signedness> {
        match name {
            "u" => Some(Signedness::Unsigned),
            "i" => Some(Signedness::Signed),
            _ => None,
        }
    }

    /// The integer type spelled `u<34>` or `i<34>` with a literal width, resolved by name alone
    /// for the sites that read a numeric type before elaboration: the type of an associated
    /// constant and the kind of a numeric generic. A width that is not a plain literal, or is
    /// not one the language has, is not resolved here.
    pub fn integer_family_with_literal_width(name: &str, args: &GenericTypeArgs) -> Option<Type> {
        let signedness = Self::integer_family(name)?;
        let [width] = args.ordered_args.as_slice() else { return None };
        if !args.named_args.is_empty() {
            return None;
        }
        let UnresolvedTypeData::Expression(UnresolvedTypeExpression::Constant(
            bits,
            None | Some(IntegerTypeSuffix::U32),
            _,
        )) = &width.typ
        else {
            return None;
        };
        let bits = u32::try_from(bits).ok()?;
        is_legal_integer_width(bits).then(|| Type::integer(signedness, bits))
    }

    pub fn to_type(self) -> Type {
        match self {
            Self::Bool => Type::Bool,
            Self::CtString => Type::Quoted(QuotedType::CtString),
            Self::Expr => Type::Quoted(QuotedType::Expr),
            Self::Fmtstr => Type::FmtString(Box::new(Type::Error), Box::new(Type::Error)),
            Self::Field => Type::FieldElement,
            Self::FunctionDefinition => Type::Quoted(QuotedType::FunctionDefinition),
            Self::Integer(signedness, bits) => Type::integer(signedness, bits),
            Self::Location => Type::Quoted(QuotedType::Location),
            Self::Module => Type::Quoted(QuotedType::Module),
            Self::Quoted => Type::Quoted(QuotedType::Quoted),
            Self::Str => Type::String(Box::new(Type::Error)),
            Self::TraitConstraint => Type::Quoted(QuotedType::TraitConstraint),
            Self::TraitDefinition => Type::Quoted(QuotedType::TraitDefinition),
            Self::TraitImpl => Type::Quoted(QuotedType::TraitImpl),
            Self::TypeDefinition => Type::Quoted(QuotedType::TypeDefinition),
            Self::TypedExpr => Type::Quoted(QuotedType::TypedExpr),
            Self::Type => Type::Quoted(QuotedType::Type),
            Self::UnresolvedType => Type::Quoted(QuotedType::UnresolvedType),
        }
    }

    /// Inverse of `to_type()`: converts a `Type` back to a `PrimitiveType` if possible.
    /// An integer type whose width still names a generic has no primitive name.
    pub fn from_type(typ: &Type) -> Option<Self> {
        match typ {
            Type::Bool => Some(Self::Bool),
            Type::FieldElement => Some(Self::Field),
            Type::Integer(signedness, width) => {
                Some(Self::Integer(*signedness, width.constant_width()?))
            }
            Type::String(_) => Some(Self::Str),
            Type::FmtString(_, _) => Some(Self::Fmtstr),
            Type::Quoted(QuotedType::CtString) => Some(Self::CtString),
            Type::Quoted(QuotedType::Expr) => Some(Self::Expr),
            Type::Quoted(QuotedType::FunctionDefinition) => Some(Self::FunctionDefinition),
            Type::Quoted(QuotedType::Location) => Some(Self::Location),
            Type::Quoted(QuotedType::Module) => Some(Self::Module),
            Type::Quoted(QuotedType::Quoted) => Some(Self::Quoted),
            Type::Quoted(QuotedType::TraitConstraint) => Some(Self::TraitConstraint),
            Type::Quoted(QuotedType::TraitDefinition) => Some(Self::TraitDefinition),
            Type::Quoted(QuotedType::TraitImpl) => Some(Self::TraitImpl),
            Type::Quoted(QuotedType::TypeDefinition) => Some(Self::TypeDefinition),
            Type::Quoted(QuotedType::TypedExpr) => Some(Self::TypedExpr),
            Type::Quoted(QuotedType::Type) => Some(Self::Type),
            Type::Quoted(QuotedType::UnresolvedType) => Some(Self::UnresolvedType),
            _ => None,
        }
    }

    pub fn to_integer_or_field(self) -> Option<Type> {
        match self {
            Self::Integer(signedness, bits) => Some(Type::integer(signedness, bits)),
            Self::Field => Some(Type::FieldElement),
            Self::Bool
            | Self::CtString
            | Self::Expr
            | Self::Fmtstr
            | Self::FunctionDefinition
            | Self::Location
            | Self::Module
            | Self::Quoted
            | Self::Str
            | Self::TraitConstraint
            | Self::TraitDefinition
            | Self::TraitImpl
            | Self::TypeDefinition
            | Self::TypedExpr
            | Self::Type
            | Self::UnresolvedType => None,
        }
    }

    pub fn name(&self) -> String {
        match self {
            Self::Bool => "bool".to_string(),
            Self::CtString => "CtString".to_string(),
            Self::Expr => "Expr".to_string(),
            Self::Field => "Field".to_string(),
            Self::Fmtstr => "fmtstr".to_string(),
            Self::FunctionDefinition => "FunctionDefinition".to_string(),
            Self::Integer(signedness, bits) => format!("{}{bits}", signedness.type_name_prefix()),
            Self::Location => "Location".to_string(),
            Self::Module => "Module".to_string(),
            Self::Quoted => "Quoted".to_string(),
            Self::Str => "str".to_string(),
            Self::TraitConstraint => "TraitConstraint".to_string(),
            Self::TraitDefinition => "TraitDefinition".to_string(),
            Self::TraitImpl => "TraitImpl".to_string(),
            Self::TypeDefinition => "TypeDefinition".to_string(),
            Self::TypedExpr => "TypedExpr".to_string(),
            Self::Type => "Type".to_string(),
            Self::UnresolvedType => "UnresolvedType".to_string(),
        }
    }
}

impl Elaborator<'_> {
    #[tracing::instrument(level = "trace", skip_all)]
    pub(crate) fn instantiate_primitive_type(
        &mut self,
        primitive_type: PrimitiveType,
        args: GenericTypeArgs,
        location: Location,
        wildcard_allowed: WildcardAllowed,
    ) -> Type {
        match primitive_type {
            PrimitiveType::Bool
            | PrimitiveType::CtString
            | PrimitiveType::Expr
            | PrimitiveType::Field
            | PrimitiveType::FunctionDefinition
            | PrimitiveType::Integer(..)
            | PrimitiveType::Location
            | PrimitiveType::Module
            | PrimitiveType::Quoted
            | PrimitiveType::TraitConstraint
            | PrimitiveType::TraitDefinition
            | PrimitiveType::TraitImpl
            | PrimitiveType::TypeDefinition
            | PrimitiveType::TypedExpr
            | PrimitiveType::Type
            | PrimitiveType::UnresolvedType => {
                if !args.is_empty() {
                    let found = args.ordered_args.len() + args.named_args.len();
                    self.push_err(CompilationError::TypeError(
                        TypeCheckError::GenericCountMismatch {
                            item: primitive_type.name(),
                            expected: 0,
                            found,
                            location,
                        },
                    ));
                }
            }
            PrimitiveType::Str => {
                let item = StrPrimitiveType;
                let (mut args, _) = self.resolve_type_args_inner(
                    args,
                    item,
                    location,
                    PathResolutionMode::MarkAsReferenced,
                    wildcard_allowed,
                );
                assert_eq!(args.len(), 1, "str generics should be: [length]");
                let length = args.pop().unwrap();
                return Type::String(Box::new(length));
            }
            PrimitiveType::Fmtstr => {
                let item = FmtstrPrimitiveType;
                let (mut args, _) = self.resolve_type_args_inner(
                    args,
                    item,
                    location,
                    PathResolutionMode::MarkAsReferenced,
                    wildcard_allowed,
                );
                assert_eq!(args.len(), 2, "fmtstr generics should be: [length, element]");
                let element = args.pop().unwrap();
                let length = args.pop().unwrap();
                return Type::FmtString(Box::new(length), Box::new(element));
            }
        }

        primitive_type.to_type()
    }

    /// Instantiates `u<N>` or `i<N>`, the parametric spelling of an integer type, from its one
    /// generic argument, the width. `u34` and `u<34>` are the same type.
    #[tracing::instrument(level = "trace", skip_all)]
    pub(crate) fn instantiate_integer_family(
        &mut self,
        signedness: Signedness,
        args: GenericTypeArgs,
        location: Location,
        wildcard_allowed: WildcardAllowed,
    ) -> Type {
        let item = IntegerPrimitiveType(signedness);
        let (mut args, _) = self.resolve_type_args_inner(
            args,
            item,
            location,
            PathResolutionMode::MarkAsReferenced,
            wildcard_allowed,
        );
        assert_eq!(args.len(), 1, "integer generics should be: [width]");
        let width = args.pop().unwrap();

        // A width that is already a number is checked here, so `u<10>` fails where it is
        // written; one that still names a generic is checked once monomorphization binds it.
        if let Some(bits) = width.constant_width()
            && !is_legal_integer_width(bits)
        {
            self.push_err(ResolverError::UnsupportedIntegerWidth { location, signedness, bits });
            return Type::Error;
        }
        Type::Integer(signedness, Box::new(width))
    }

    /// Instantiates a primitive type with turbofish generics.
    ///
    /// # Returns
    /// A tuple of:
    /// - The instantiated [Type]
    /// - A boolean indicating whether this primitive type has generics
    #[tracing::instrument(level = "trace", skip_all)]
    pub(crate) fn instantiate_primitive_type_with_turbofish(
        &mut self,
        primitive_type: PrimitiveType,
        turbofish: Option<Turbofish>,
        errors: &mut Vec<CompilationError>,
    ) -> (Type, bool) {
        match primitive_type {
            PrimitiveType::Bool
            | PrimitiveType::CtString
            | PrimitiveType::Expr
            | PrimitiveType::Field
            | PrimitiveType::FunctionDefinition
            | PrimitiveType::Integer(..)
            | PrimitiveType::Location
            | PrimitiveType::Module
            | PrimitiveType::Quoted
            | PrimitiveType::TraitConstraint
            | PrimitiveType::TraitDefinition
            | PrimitiveType::TraitImpl
            | PrimitiveType::TypeDefinition
            | PrimitiveType::TypedExpr
            | PrimitiveType::Type
            | PrimitiveType::UnresolvedType => {
                if let Some(turbofish) = turbofish {
                    errors.push(CompilationError::TypeError(
                        TypeCheckError::GenericCountMismatch {
                            item: primitive_type.name(),
                            expected: 0,
                            found: turbofish.generics.len(),
                            location: turbofish.location,
                        },
                    ));
                }
                (primitive_type.to_type(), false)
            }
            PrimitiveType::Str => {
                let item = StrPrimitiveType;
                let item_generic_kinds = item.generic_kinds(self.interner);
                let generics = vecmap(&item_generic_kinds, |kind| {
                    self.interner.next_type_variable_with_kind(kind.clone())
                });
                let mut args = if let Some(turbofish) = turbofish {
                    self.resolve_item_turbofish_generics(
                        item.item_kind(),
                        &item.item_name(self.interner),
                        item_generic_kinds,
                        generics,
                        Some(turbofish.generics),
                        turbofish.location,
                        errors,
                    )
                } else {
                    generics
                };
                assert_eq!(args.len(), 1, "str generics should be: [length]");
                let length = args.pop().unwrap();
                (Type::String(Box::new(length)), true)
            }
            PrimitiveType::Fmtstr => {
                let item = FmtstrPrimitiveType;
                let item_generic_kinds = item.generic_kinds(self.interner);
                let generics = vecmap(&item_generic_kinds, |kind| {
                    self.interner.next_type_variable_with_kind(kind.clone())
                });
                let mut args = if let Some(turbofish) = turbofish {
                    self.resolve_item_turbofish_generics(
                        FmtstrPrimitiveType.item_kind(),
                        &item.item_name(self.interner),
                        item_generic_kinds,
                        generics,
                        Some(turbofish.generics),
                        turbofish.location,
                        errors,
                    )
                } else {
                    generics
                };
                assert_eq!(args.len(), 2, "fmtstr generics should be: [length, element]");
                let element = args.pop().unwrap();
                let length = args.pop().unwrap();
                (Type::FmtString(Box::new(length), Box::new(element)), true)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PrimitiveType;
    use crate::shared::Signedness;

    #[test]
    fn from_type_to_type() {
        for primitive in PrimitiveType::NAMED {
            let typ = primitive.to_type();
            let recovered = PrimitiveType::from_type(&typ);
            assert_eq!(
                recovered,
                Some(primitive),
                "from_type(to_type({primitive:?})) should roundtrip"
            );
        }
    }

    #[test]
    fn to_type_from_type() {
        for primitive in PrimitiveType::NAMED {
            let typ = primitive.to_type();
            let recovered = PrimitiveType::from_type(&typ).unwrap();
            let typ2 = recovered.to_type();
            assert_eq!(typ, typ2, "to_type(from_type(to_type({primitive:?}))) should roundtrip");
        }
    }

    #[test]
    fn every_named_primitive_resolves_to_itself_by_name() {
        for primitive in PrimitiveType::NAMED {
            assert_eq!(PrimitiveType::lookup_by_name(&primitive.name()), Some(primitive));
        }
    }

    #[test]
    fn integer_names_resolve_at_every_legal_width_and_nowhere_else() {
        for (name, expected) in [
            ("u8", Some((Signedness::Unsigned, 8))),
            ("i128", Some((Signedness::Signed, 128))),
            ("u34", Some((Signedness::Unsigned, 34))),
            ("i512", Some((Signedness::Signed, 512))),
            ("u65536", Some((Signedness::Unsigned, 65536))),
            ("u65538", None),
            ("u10", None),
            ("u24", None),
            ("u1", None),
            ("u0", None),
            ("u007", None),
            ("u", None),
            ("i", None),
            ("u4294967296", None),
        ] {
            let expected =
                expected.map(|(signedness, bits)| PrimitiveType::Integer(signedness, bits));
            assert_eq!(PrimitiveType::lookup_by_name(name), expected, "{name}");
        }
    }

    #[test]
    fn the_integer_families_are_only_u_and_i() {
        assert_eq!(PrimitiveType::integer_family("u"), Some(Signedness::Unsigned));
        assert_eq!(PrimitiveType::integer_family("i"), Some(Signedness::Signed));
        for name in ["u8", "str", "U", "uint", ""] {
            assert_eq!(PrimitiveType::integer_family(name), None, "{name}");
        }
    }

    #[test]
    fn a_generic_width_has_no_primitive_name() {
        let width = crate::Type::type_variable(crate::TypeVariableId(0));
        let typ = crate::Type::Integer(Signedness::Unsigned, Box::new(width));
        assert_eq!(PrimitiveType::from_type(&typ), None);
    }
}
