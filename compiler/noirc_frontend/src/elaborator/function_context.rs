//! Function-local context management for type variables and trait constraints.

use noirc_errors::Location;
use num_bigint::BigInt;

use crate::{
    Kind, Type, TypeBindings, TypeVariable,
    elaborator::lints::check_integer_literal_fits_its_type,
    hir::{
        comptime::{InterpreterError, Value},
        type_check::{FieldCastSuggestion, NoMatchingImplFoundError, TypeCheckError},
    },
    hir_def::{
        expr::{HirExpression, HirLiteral},
        traits::TraitConstraint,
    },
    node_interner::{
        DefinitionKind, ExprId, GlobalValue, ImplSearchErrorKind, TraitImplKind, TraitLookupMode,
        TypeId,
    },
    shared::MIN_INTEGER_WIDTH,
};
use crate::{TypeVariableId, node_interner::DefinitionId};

use super::Elaborator;

#[derive(Default)]
pub(super) struct FunctionContext {
    /// All type variables created in the current function.
    /// This map is used to default any integer type variables at the end of
    /// a function (before checking trait constraints) if a type wasn't already chosen.
    defaultable_type_variables: Vec<Type>,

    /// Type variables that must be bound at the end of the function.
    required_type_variables: Vec<RequiredTypeVariable>,

    /// Trait constraints are collected during type checking until they are
    /// verified at the end of a function. This is because constraints arise
    /// on each variable, but it is only until function calls when the types
    /// needed for the trait constraint may become known.
    /// The `select impl` bool indicates whether, after verifying the trait constraint,
    /// the resulting trait impl should be the one used for a call (sometimes trait
    /// constraints are verified but there's no call associated with them, like in the
    /// case of checking generic arguments)
    trait_constraints: Vec<LocalTraitConstraint>,

    /// All `ExprId` in a function that correspond to integer literals.
    /// At the end, if they don't fit in their type's min/max range, we'll produce an error.
    integer_literal_expr_ids: Vec<ExprId>,

    /// All casts to `Field` or to an integer in a function. Their source may be a type variable
    /// where the cast is written, so the rules for a cast between `Field` and an integer are
    /// applied at the end, once it is bound.
    field_casts: Vec<FieldCast>,
}

/// A cast that may be between `Field` and an integer, kept until its source type is known.
#[derive(Debug)]
struct FieldCast {
    lhs: ExprId,
    from: Type,
    to: Type,
    location: Location,
    /// True when nothing but defaulting gives the source its type.
    defaulted: bool,
}

/// A type variable that is required to be bound after type-checking a function.
#[derive(Debug)]
struct RequiredTypeVariable {
    type_variable_id: TypeVariableId,
    typ: Type,
    kind: BindableTypeVariableKind,
    location: Location,
}

/// A constraint local to the current [`FunctionContext`] to solve at the end of the context.
#[derive(Debug)]
struct LocalTraitConstraint {
    constraint: TraitConstraint,

    /// The expression this constraint originated from. Used for its location in error messages,
    /// and if `select_impl` is true, to be the expression the impl will replace during
    /// monomorphization.
    expr: ExprId,

    /// True if a reference to some function in this constraint's trait should replace the
    /// expression at `self.expr` during monomorphization.
    select_impl: bool,

    /// Where [`Elaborator::select_where_possible`] has got with this constraint.
    progress: Progress,

    /// For a where-clause bound of a referenced function that
    /// [`Elaborator::check_where_clauses_without_generics`] also checks where it is declared, the
    /// bound as declared, before the reference's instantiation. A failure here is not reported
    /// again if that check reports the declared bound. It does not decide every instance (an
    /// associated type set to a generic is only fixed here), so any other failure is reported.
    declared_bound: Option<TraitConstraint>,
}

/// How far solving a queued constraint while its function is being checked has got. Whatever
/// it reaches, the constraint is checked again at the end of the function, which reports errors
/// and records the impl it selects.
#[derive(Debug, Default)]
enum Progress {
    /// Not tried yet.
    #[default]
    New,
    /// Tried while the impl to use was undecided. The type variables it was waiting on, so it is
    /// only tried again once one of them is bound.
    Stalled(Vec<TypeVariable>),
    /// An impl was selected and its bindings applied.
    Selected,
}

impl Progress {
    /// Whether trying the constraint again could get further: it hasn't been tried, or one of
    /// the type variables it was waiting on has been bound since.
    fn may_advance(&self) -> bool {
        match self {
            Progress::New => true,
            Progress::Stalled(waiting_on) => {
                waiting_on.iter().any(|var| !var.binding().is_unbound())
            }
            Progress::Selected => false,
        }
    }
}

/// The kind of required type variable.
#[derive(Debug, Copy, Clone)]
pub(super) enum BindableTypeVariableKind {
    /// The type variable corresponds to a struct generic, in a constructor.
    StructGeneric { struct_id: TypeId, index: usize },
    /// The type variable corresponds to the type of an array literal.
    ArrayLiteral { is_array: bool },
    /// The type variable corresponds to an identifier, whose definition ID is the given one.
    Ident(DefinitionId),
}

impl Elaborator<'_> {
    /// Push a type variable into the current `FunctionContext` to be defaulted if needed
    /// at the end of the earlier of either the current function or the current comptime scope.
    #[tracing::instrument(level = "trace", skip_all)]
    pub(super) fn push_defaultable_type_variable(&mut self, typ: Type) {
        self.get_function_context_mut().defaultable_type_variables.push(typ);
    }

    /// Push a type variable (its ID and type) as a required type variable: it must be
    /// bound after type-checking the current function.
    ///
    /// The type variable is only pushed if the elaborator is not in a comptime context.
    /// The reason is that in a comptime context the type of a variable might change
    /// across a loop's iterations, so a type can temporarily remain as `Type<_>` where
    /// `_` is bound by the interpreter evaluating an expression's type being unified with
    /// that type.
    #[tracing::instrument(level = "trace", skip_all)]
    pub(super) fn push_required_type_variable(
        &mut self,
        type_variable_id: TypeVariableId,
        typ: Type,
        kind: BindableTypeVariableKind,
        location: Location,
    ) {
        if !self.in_comptime_context() {
            let var = RequiredTypeVariable { type_variable_id, typ, kind, location };
            self.get_function_context_mut().required_type_variables.push(var);
        }
    }

    /// Push a trait constraint into the current `FunctionContext` to be solved if needed
    /// at the end of the earlier of either the current function or the current comptime scope.
    #[tracing::instrument(level = "trace", skip_all)]
    pub(super) fn push_trait_constraint(
        &mut self,
        constraint: TraitConstraint,
        expr: ExprId,
        select_impl: bool,
    ) {
        let declared_bound = None;
        self.push_local_trait_constraint(constraint, expr, select_impl, declared_bound);
    }

    /// [`Self::push_trait_constraint`] for a bound of the where clause of the function referenced
    /// by `expr`, instantiated for that reference.
    pub(super) fn push_where_clause_constraint(
        &mut self,
        constraint: TraitConstraint,
        expr: ExprId,
        declared_bound: Option<TraitConstraint>,
    ) {
        let select_impl = false;
        self.push_local_trait_constraint(constraint, expr, select_impl, declared_bound);
    }

    fn push_local_trait_constraint(
        &mut self,
        constraint: TraitConstraint,
        expr: ExprId,
        select_impl: bool,
        declared_bound: Option<TraitConstraint>,
    ) {
        self.get_function_context_mut().trait_constraints.push(LocalTraitConstraint {
            constraint,
            expr,
            select_impl,
            progress: Progress::New,
            declared_bound,
        });
    }

    /// Follows the bindings of `typ`, which is about to be inspected. If it isn't known yet, the
    /// queued trait constraints that can be solved are solved first, as one of them may decide it
    /// (the result of a call that returns an associated type).
    pub(super) fn structurally_resolve_type(&mut self, typ: &Type) -> Type {
        let typ = typ.follow_bindings();
        if !matches!(typ, Type::TypeVariable(_)) {
            return typ;
        }
        self.select_where_possible();
        typ.follow_bindings()
    }

    /// Solves each queued trait constraint whose impl can already be decided, binding the type
    /// variables that impl decides (its associated types, and the trait arguments it fixes). A
    /// constraint whose impl still depends on an unknown type is left alone, and only tried
    /// again once one of its types has changed.
    pub(super) fn select_where_possible(&mut self) {
        let context = self.function_context.last_mut().expect("no function context");
        let mut to_try = Vec::new();
        for (index, local) in context.trait_constraints.iter().enumerate() {
            if local.progress.may_advance() {
                to_try.push((index, local.constraint.clone()));
            }
        }
        for (index, constraint) in to_try {
            let bound = &constraint.trait_bound;
            let progress = match self.interner.try_lookup_trait_implementation(
                &constraint.typ,
                bound.trait_id,
                &bound.trait_generics.ordered,
                &bound.trait_generics.named,
                TraitLookupMode::Default,
            ) {
                Ok((_, bindings, _)) => {
                    Type::apply_type_bindings(bindings);
                    Progress::Selected
                }
                // Errors are reported by the check at the end of the function. A constraint whose
                // types can't change any more is never tried again.
                Err(_) => Progress::Stalled(unbound_type_variables(&constraint)),
            };
            self.get_function_context_mut().trait_constraints[index].progress = progress;
        }
    }

    /// Push an `ExprId` that corresponds to an integer literal.
    /// At the end of the current function we'll check that they fit in their type's range.
    #[tracing::instrument(level = "trace", skip_all)]
    pub(crate) fn push_integer_literal_expr_id(&mut self, literal_expr_id: ExprId) {
        self.get_function_context_mut().integer_literal_expr_ids.push(literal_expr_id);
    }

    /// Push a cast of the expression `lhs`, of type `from`, to `Field` or to an integer type.
    /// At the end of the current function we'll check that the cast may be written.
    pub(super) fn push_field_cast(
        &mut self,
        lhs: ExprId,
        from: Type,
        to: Type,
        location: Location,
    ) {
        let cast = FieldCast { lhs, from, to, location, defaulted: false };
        self.get_function_context_mut().field_casts.push(cast);
    }

    #[tracing::instrument(level = "trace", skip_all)]
    fn get_function_context_mut(&mut self) -> &mut FunctionContext {
        let context = self.function_context.last_mut();
        context.expect("The function_context stack should always be non-empty")
    }

    #[tracing::instrument(level = "trace", skip_all)]
    pub(super) fn push_function_context(&mut self) {
        self.function_context.push(FunctionContext::default());
    }

    /// Defaults all type variables used in this function context then solves
    /// all still-unsolved trait constraints in this context.
    #[tracing::instrument(level = "trace", skip_all)]
    pub(super) fn check_and_pop_function_context(&mut self) {
        let unbound_casts = self.pop_function_context();
        self.check_unbound_field_casts(unbound_casts);
    }

    /// Like [Self::check_and_pop_function_context], for a context nested in a function's own:
    /// a cast there may name a variable of the function, which is bound only later, so such a
    /// cast is checked when the enclosing context ends.
    #[tracing::instrument(level = "trace", skip_all)]
    pub(super) fn check_and_pop_nested_function_context(&mut self) {
        let unbound_casts = self.pop_function_context();
        // The bottom context belongs to no function, so nothing there binds a variable later.
        if self.function_context.len() > 1 {
            self.get_function_context_mut().field_casts.extend(unbound_casts);
        } else {
            self.check_unbound_field_casts(unbound_casts);
        }
    }

    /// Checks and pops the current context, returning the casts whose source is still a type
    /// variable.
    fn pop_function_context(&mut self) -> Vec<FieldCast> {
        let context = self.function_context.pop().expect("Imbalanced function_context pushes");
        self.bind_type_variables_from_trait_constraints(&context.trait_constraints);
        // Marked before defaulting, which is what binds a source nothing else gives a type.
        let mut field_casts = context.field_casts;
        for cast in &mut field_casts {
            cast.defaulted = matches!(cast.from.follow_bindings(), Type::TypeVariable(_));
        }
        self.check_defaultable_type_variables(context.defaultable_type_variables);
        let unbound_casts = self.check_field_casts(field_casts);
        self.check_integer_literal_fit_their_type(context.integer_literal_expr_ids);
        self.check_trait_constraints(context.trait_constraints);
        self.check_required_type_variables(context.required_type_variables);
        unbound_casts
    }

    /// Best-effort trait-constraint resolution that runs before integer-literal defaulting.
    ///
    /// When a constraint has exactly one impl that unifies with the current types,
    /// `find_impl` succeeds and auto-applies its type bindings — including binding
    /// `IntegerOrField` type variables to whatever the impl requires (e.g. `i32`).
    /// Defaulting then becomes a no-op for those variables, so the real
    /// [`Self::check_trait_constraints`] pass sees the impl-chosen type rather than
    /// the defaulted `Field`.
    ///
    /// On failure (no match, multiple matches, or insufficient annotations), no
    /// bindings are applied and no error is reported — the constraint is left
    /// for the post-defaulting pass, which is responsible for the final verdict
    /// and error reporting.
    #[tracing::instrument(level = "trace", skip_all)]
    fn bind_type_variables_from_trait_constraints(
        &self,
        trait_constraints: &[LocalTraitConstraint],
    ) {
        let current_trait_self = self.item.impl_context.trait_self_type();
        for local in trait_constraints {
            let _ = local.constraint.find_impl(self.interner, current_trait_self.as_ref());
        }
    }

    fn check_defaultable_type_variables(&self, type_variables: Vec<Type>) {
        for typ in type_variables {
            if let Type::TypeVariable(variable) = typ.follow_bindings() {
                let msg = "TypeChecker should only track defaultable type vars";
                variable.bind(variable.kind().default_type().expect(msg));
            }
        }
    }

    #[tracing::instrument(level = "trace", skip_all)]
    fn check_integer_literal_fit_their_type(&mut self, expr_ids: Vec<ExprId>) {
        for expr_id in expr_ids {
            if let Some(error) = check_integer_literal_fits_its_type(self.interner, &expr_id) {
                self.push_err(error);
            }
        }
    }

    /// Applies the rules for a cast between `Field` and an integer to each cast whose source
    /// type is known, and returns the rest.
    fn check_field_casts(&mut self, casts: Vec<FieldCast>) -> Vec<FieldCast> {
        let mut unbound = Vec::new();
        for cast in casts {
            let from = cast.from.follow_bindings();
            if matches!(from, Type::TypeVariable(_)) {
                unbound.push(cast);
            } else if let Some(error) = self.field_cast_error(&cast, from) {
                self.push_err(error);
            }
        }
        unbound
    }

    /// Applies the same rules to casts whose source no context binds, which makes it the
    /// default of its kind.
    fn check_unbound_field_casts(&mut self, casts: Vec<FieldCast>) {
        for cast in casts {
            if let Type::TypeVariable(variable) = cast.from.follow_bindings()
                && let Some(from) = variable.kind().default_type()
                && let Some(error) = self.field_cast_error(&cast, from)
            {
                self.push_err(error);
            }
        }
    }

    /// The error of writing `cast` from a value of type `from`, if it crosses between `Field`
    /// and an integer. A value crosses through `From`, `TryFrom` or `UnsafeFrom`, which say what
    /// happens to one that does not fit. A literal that fits is the same number under every
    /// field.
    fn field_cast_error(&self, cast: &FieldCast, from: Type) -> Option<TypeCheckError> {
        let to_field = matches!(cast.to, Type::FieldElement);
        let crosses = match from {
            Type::Integer(..) => to_field,
            Type::FieldElement => !to_field && !self.is_literal_that_fits(cast),
            _ => false,
        };
        crosses.then(|| {
            let to = cast.to.clone();
            let suggestion = self.field_cast_suggestion(cast, &from);
            TypeCheckError::FieldCastDenied { from, to, suggestion, location: cast.location }
        })
    }

    /// The value of the integer literal `cast` is written on, if it is one.
    fn cast_literal(&self, cast: &FieldCast) -> Option<BigInt> {
        match self.interner.expression(&cast.lhs) {
            HirExpression::Literal(HirLiteral::Integer(value)) => Some(value),
            _ => None,
        }
    }

    /// Whether `cast` is written on a literal that its target type holds as written, at every
    /// width the target may have where its width names a generic.
    fn is_literal_that_fits(&self, cast: &FieldCast) -> bool {
        let maximum = match &cast.to {
            Type::Integer(sign, width) if width.constant_width().is_none() => {
                Type::integer(*sign, MIN_INTEGER_WIDTH).integral_maximum_size()
            }
            to => to.integral_maximum_size(),
        };
        self.cast_literal(cast).is_some_and(|value| {
            value >= BigInt::ZERO && maximum.is_some_and(|maximum| value <= maximum)
        })
    }

    /// What to write in place of `cast`, which crosses between `Field` and an integer from a
    /// value of type `from`.
    fn field_cast_suggestion(&self, cast: &FieldCast, from: &Type) -> FieldCastSuggestion {
        if matches!(cast.to, Type::FieldElement) {
            if self.field_from_exists(from) {
                FieldCastSuggestion::FieldFrom
            } else {
                FieldCastSuggestion::Convert
            }
        } else if let Some(literal) = self.typed_literal(cast) {
            FieldCastSuggestion::TypedLiteral(literal)
        } else if cast.defaulted && self.cast_literal(cast).is_none() {
            FieldCastSuggestion::DeclareType(self.cast_variable_name(cast))
        } else {
            FieldCastSuggestion::Convert
        }
    }

    /// Whether the standard library's `From` converts every value of `from` to `Field`.
    fn field_from_exists(&self, from: &Type) -> bool {
        let from_trait = self.interner.traits.iter().find(|(trait_id, the_trait)| {
            trait_id.0.krate.is_stdlib() && the_trait.name.as_str() == "From"
        });
        from_trait.is_some_and(|(trait_id, _)| {
            let field = Type::FieldElement;
            let generics = [from.clone()];
            let mode = TraitLookupMode::Default;
            self.interner
                .try_lookup_trait_implementation(&field, *trait_id, &generics, &[], mode)
                .is_ok()
        })
    }

    /// The name of the variable `cast` is written on, if it is one.
    fn cast_variable_name(&self, cast: &FieldCast) -> Option<String> {
        match self.interner.expression(&cast.lhs) {
            HirExpression::Ident(ident, _) => {
                Some(self.interner.definition_name(ident.id).to_owned())
            }
            _ => None,
        }
    }

    /// The literal `cast` is written on, typed as its target: what to write in place of a
    /// negative literal, which a cast would take through `Field`.
    fn typed_literal(&self, cast: &FieldCast) -> Option<String> {
        let value = self.cast_literal(cast)?;
        let minimum = cast.to.integral_minimum_size()?;
        (value < BigInt::ZERO && value >= minimum).then(|| format!("{value}{}", cast.to))
    }

    #[tracing::instrument(level = "trace", skip_all)]
    fn check_trait_constraints(&mut self, trait_constraints: Vec<LocalTraitConstraint>) {
        let current_trait_self = self.item.impl_context.trait_self_type();

        for local in trait_constraints {
            match local.constraint.find_impl(self.interner, current_trait_self.as_ref()) {
                Ok((impl_kind, instantiation_bindings)) => {
                    if local.select_impl {
                        self.select_impl(local.expr, impl_kind, instantiation_bindings);
                    }
                }
                Err(_) if self.reported_at_declaration(local.declared_bound.as_ref()) => (),
                Err(error) => {
                    let location = self.interner.expr_location(&local.expr);
                    self.push_trait_constraint_error(&local.constraint.typ, error, location);
                }
            }
        }
    }

    /// Whether `declared_bound`, a where-clause bound as declared, is reported as failing where it
    /// is declared (see [`Elaborator::check_where_clauses_without_generics`]).
    fn reported_at_declaration(&self, declared_bound: Option<&TraitConstraint>) -> bool {
        declared_bound
            .is_some_and(|bound| self.where_clause_bound_without_generics_error(bound).is_some())
    }

    #[tracing::instrument(level = "trace", skip_all)]
    fn select_impl(
        &mut self,
        function_ident_id: ExprId,
        impl_kind: TraitImplKind,
        instantiation_bindings: TypeBindings,
    ) {
        // Insert any additional instantiation bindings into this expression's
        // instantiation bindings. We should avoid doing this if `select_impl` is
        // not true since that means we're not solving for this expressions exact
        // impl anyway. If we ignore this, we may rarely overwrite existing type
        // bindings causing incorrect types. The `vector_regex` test is one example
        // of that happening without this being behind `select_impl`.
        let mut bindings = self.interner.get_instantiation_bindings(function_ident_id).clone();

        // These can clash in the `vector_regex` test which causes us to insert
        // incorrect type bindings if they override the previous bindings.
        for (id, binding) in instantiation_bindings {
            let existing = bindings.insert(id, binding.clone());

            if let Some((_, type_var, existing)) = existing {
                let existing = existing.follow_bindings();
                let new = binding.2.follow_bindings();

                // Exact equality on types is intentional here, we never want to
                // overwrite even type variables but should probably avoid a panic if
                // the types are exactly the same.
                if existing != new {
                    panic!(
                        "Overwriting an existing type binding with a different type!\n  {type_var:?} <- {existing:?}\n  {type_var:?} <- {new:?}"
                    );
                }
            }
        }

        self.interner.store_instantiation_bindings(function_ident_id, bindings);
        self.interner.select_impl_for_expression(function_ident_id, impl_kind);
    }

    #[tracing::instrument(level = "trace", skip_all)]
    pub(super) fn push_trait_constraint_error(
        &mut self,
        object_type: &Type,
        error: ImplSearchErrorKind,
        location: Location,
    ) {
        // An object of the error type comes from an error that has already been reported.
        if matches!(object_type.follow_bindings(), Type::Error) {
            return;
        }

        match error {
            ImplSearchErrorKind::TypeAnnotationsNeededOnObjectType => {
                self.push_err(TypeCheckError::TypeAnnotationsNeededForMethodCall { location });
            }
            ImplSearchErrorKind::NoImplFound(constraints)
            | ImplSearchErrorKind::NoMatching(constraints) => {
                if let Some(error) =
                    NoMatchingImplFoundError::new(self.interner, constraints, location)
                {
                    self.push_err(TypeCheckError::NoMatchingImplFound(error));
                }
            }
            ImplSearchErrorKind::RecursionLimitReached => {
                self.push_err(InterpreterError::TraitImplResolutionRecursionLimitReached {
                    location,
                });
            }
            ImplSearchErrorKind::MultipleMatching(candidates) => {
                let object_type = object_type.clone();
                let err =
                    TypeCheckError::MultipleMatchingImpls { object_type, location, candidates };
                self.push_err(err);
            }
        }
    }

    #[tracing::instrument(level = "trace", skip_all)]
    fn check_required_type_variables(&mut self, type_variables: Vec<RequiredTypeVariable>) {
        for var in type_variables {
            let id = var.type_variable_id;
            if let Type::TypeVariable(_) = var.typ.follow_bindings() {
                let location = var.location;
                match var.kind {
                    BindableTypeVariableKind::StructGeneric { struct_id, index } => {
                        let data_type = self.interner.get_type(struct_id);
                        let generic = &data_type.borrow().generics[index];
                        let generic_name = generic.name.to_string();
                        let item_kind = "struct";
                        let item_name = data_type.borrow().name.to_string();
                        let is_numeric = matches!(*generic.type_var.kind(), Kind::Numeric(..));
                        self.push_err(TypeCheckError::TypeAnnotationNeededOnItem {
                            location,
                            generic_name,
                            item_kind,
                            item_name,
                            is_numeric,
                        });
                    }
                    BindableTypeVariableKind::ArrayLiteral { is_array } => {
                        self.push_err(TypeCheckError::TypeAnnotationNeededOnArrayLiteral {
                            location,
                            is_array,
                        });
                    }
                    BindableTypeVariableKind::Ident(definition_id) => {
                        let definition = self.interner.definition(definition_id);
                        let definition_kind = definition.kind.clone();
                        match definition_kind {
                            DefinitionKind::Function(func_id) => {
                                let (direct_generics_clone, self_type_clone, all_generics_clone) =
                                    self.with_function_meta(func_id, |meta| {
                                        (
                                            meta.direct_generics.clone(),
                                            meta.self_type.clone(),
                                            meta.all_generics.clone(),
                                        )
                                    });

                                // Try to find the type variable in the function's generic arguments
                                let generic = direct_generics_clone
                                    .iter()
                                    .find(|generic| generic.type_var.id() == id);
                                if let Some(generic) = generic {
                                    let item_name =
                                        self.interner.definition_name(definition_id).to_string();
                                    let is_numeric =
                                        matches!(*generic.type_var.kind(), Kind::Numeric(..));
                                    self.push_err(TypeCheckError::TypeAnnotationNeededOnItem {
                                        location,
                                        generic_name: generic.name.to_string(),
                                        item_kind: "function",
                                        item_name,
                                        is_numeric,
                                    });
                                    continue;
                                }

                                // If we find one in `all_generics` it means it's a generic on the type
                                // the function is in.
                                let Some(Type::DataType(typ, ..)) = &self_type_clone else {
                                    continue;
                                };
                                let typ = typ.borrow();
                                let item_name = typ.name.to_string();
                                let item_kind = if typ.is_struct() { "struct" } else { "enum" };
                                drop(typ);

                                let mut all_generics = all_generics_clone.iter();
                                let generic =
                                    all_generics.find(|generic| generic.type_var.id() == id);
                                if let Some(generic) = generic {
                                    let is_numeric =
                                        matches!(*generic.type_var.kind(), Kind::Numeric(..));
                                    self.push_err(TypeCheckError::TypeAnnotationNeededOnItem {
                                        location,
                                        generic_name: generic.name.to_string(),
                                        item_kind,
                                        item_name: item_name.clone(),
                                        is_numeric,
                                    });
                                }
                            }
                            DefinitionKind::Global(global_id) => {
                                // Check if this global points to an enum variant, then get the enum's generics
                                // and find the type variable there.
                                let global = self.interner.get_global(global_id);
                                let GlobalValue::Resolved(Value::Enum(_, _, Type::Forall(_, typ))) =
                                    &global.value
                                else {
                                    continue;
                                };

                                let typ: &Type = typ;
                                let Type::DataType(def, _) = typ else {
                                    continue;
                                };

                                let def = def.borrow();
                                let item_name = def.name.to_string();
                                let mut generics = def.generics.iter();
                                let generic =
                                    generics.find(|generic| generic.type_var.id() == id).cloned();
                                drop(def);
                                if let Some(generic) = generic {
                                    let is_numeric =
                                        matches!(*generic.type_var.kind(), Kind::Numeric(..));
                                    self.push_err(TypeCheckError::TypeAnnotationNeededOnItem {
                                        location,
                                        generic_name: generic.name.to_string(),
                                        item_kind: "enum",
                                        item_name: item_name.clone(),
                                        is_numeric,
                                    });
                                }
                            }
                            _ => (),
                        }
                    }
                }
            }
        }
    }
}

/// The unbound type variables in `constraint`'s types.
fn unbound_type_variables(constraint: &TraitConstraint) -> Vec<TypeVariable> {
    let mut variables = Vec::new();
    constraint.typ.unbound_type_variables(&mut variables);
    let generics = &constraint.trait_bound.trait_generics;
    for typ in &generics.ordered {
        typ.unbound_type_variables(&mut variables);
    }
    for named in &generics.named {
        named.typ.unbound_type_variables(&mut variables);
    }
    variables
}
