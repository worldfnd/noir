//! Field rules across configurations, each compiled and evaluated in one binary.

use acvm::{FieldConfig, FieldId};
use noirc_errors::CustomDiagnostic;
use num_bigint::BigUint;

use crate::hir::def_collector::dc_crate::CompilationError;
use crate::hir::type_check::TypeCheckError;
use crate::test_utils::{
    GetProgramOptions, get_monomorphized, get_monomorphized_for_field, get_program_with_options,
    stdlib_src,
};
use crate::tests::{
    assert_no_errors, check_errors, check_errors_with_options, get_program_errors_for_field,
};
use crate::validity::InvalidType;

#[test]
fn field_literals_must_be_canonical() {
    for field in FieldId::ALL {
        let modulus = FieldConfig::new(field).modulus();

        let largest = format!("fn main() {{ let _: Field = {}; }}", modulus - 1u8);
        let errors = get_program_errors_for_field(&largest, field);
        assert!(errors.is_empty(), "{field}: {errors:?}");

        for too_large in [
            format!("fn main() {{ let _: Field = {modulus}; }}"),
            format!(
                "fn value<let N: Field>() -> Field {{ N }}
                 fn main() -> pub Field {{ value::<{modulus}_Field>() }}"
            ),
        ] {
            let errors = get_program_errors_for_field(&too_large, field);
            assert!(
                errors.iter().any(|error| matches!(
                    error,
                    CompilationError::TypeError(TypeCheckError::IntegerLiteralDoesNotFitItsType { range, .. })
                        if range == &format!("0..{modulus}")
                )),
                "{field}: expected the literal `{modulus}` to be rejected, got {errors:?}"
            );
        }
    }
}

#[test]
fn integer_literals_above_the_modulus_stay_exact() {
    for field in FieldId::ALL {
        let src = format!("fn main() {{ let x: u64 = {}; assert(x != 0); }}", u64::MAX);
        let program = get_monomorphized_for_field(&src, field).unwrap().to_string();
        assert!(program.contains(&u64::MAX.to_string()), "{field}: {program}");
    }
}

/// A signed value reaches `Field` only through an unsigned cast, since `i8 as Field` is a type error.
#[test]
fn comptime_casts_wrap_to_the_target_width() {
    let src = "fn main() {
        comptime {
            let wide: u16 = 256;
            assert((wide as u8) == 0);
            let negative: i8 = -1;
            assert((negative as u8) == 255);
            assert(((negative as u8) as Field) == 255);
        }
    }";
    assert_no_errors(src);
}

/// An unsigned type is cast to `Field` only if every one of its values is below the modulus;
/// `Field` never reduces a cast. The escape hatch is an explicit narrowing cast.
#[test]
fn casts_to_field_are_refused_when_the_type_can_exceed_the_modulus() {
    for field in FieldId::ALL {
        let config = FieldConfig::new(field);
        for bits in [8u32, 16, 32, 64, 128] {
            let src = format!("fn main() {{ let x: u{bits} = 1; let _ = x as Field; }}");
            let errors = get_program_errors_for_field(&src, field);
            let refused = errors.iter().any(|error| {
                matches!(
                    error,
                    CompilationError::TypeError(TypeCheckError::IntegerTypeExceedsField { .. })
                )
            });
            if config.fits_unsigned(bits) {
                assert!(
                    errors.is_empty(),
                    "{field}: u{bits} fits below the modulus, got {errors:?}"
                );
            } else {
                assert!(refused, "{field}: u{bits} can exceed the modulus, got {errors:?}");
            }
        }
    }
}

/// The type checker runs before the comptime interpreter, so a `comptime` block is refused too.
#[test]
fn comptime_casts_to_field_follow_the_same_rule() {
    for field in FieldId::ALL {
        let accepted = FieldConfig::new(field).fits_unsigned(64);
        let src = format!(
            "fn main() {{
                comptime {{
                    let x: u64 = {};
                    let _ = x as Field;
                }}
            }}",
            u64::MAX
        );
        let errors = get_program_errors_for_field(&src, field);
        if accepted {
            assert!(errors.is_empty(), "{field}: {errors:?}");
        } else {
            assert!(
                errors.iter().any(|error| matches!(
                    error,
                    CompilationError::TypeError(TypeCheckError::IntegerTypeExceedsField { .. })
                )),
                "{field}: {errors:?}"
            );
        }
    }
}

#[test]
fn narrowing_before_a_field_cast_is_accepted_under_every_field() {
    let src = format!(
        "fn main() {{
            let x: u128 = {};
            assert(((x as u32) as Field) == {});
            comptime {{
                let x: u128 = {};
                assert(((x as u32) as Field) == {});
            }}
        }}",
        u128::MAX,
        u32::MAX,
        u128::MAX,
        u32::MAX
    );
    assert_no_errors(&src);
}

/// The type checker sees the cast while the source is still a type variable, so the rule is
/// applied again once inference has bound it, before the cast reaches the monomorphized AST.
#[test]
fn casts_to_field_through_an_inferred_type_follow_the_same_rule() {
    let src = "fn apply<T>(f: fn(T) -> Field, x: T) -> Field { f(x) }

    fn main(x: u32) {
        let _ = apply(|v| v as Field, x as u64);
    }";
    for field in FieldId::ALL {
        let result = get_monomorphized_for_field(src, field);
        if FieldConfig::new(field).fits_unsigned(64) {
            assert!(result.is_ok(), "{field}: {result:?}");
        } else {
            let error = result.expect_err("u64 can exceed the modulus");
            let diagnostic = CustomDiagnostic::from(error);
            assert!(
                diagnostic.message.contains("can exceed the field modulus"),
                "{field}: {}",
                diagnostic.message
            );
        }
    }
}

#[test]
fn signed_casts_to_field_through_an_inferred_type_are_refused() {
    let src = "fn apply<T>(f: fn(T) -> Field, x: T) -> Field { f(x) }

    fn main(x: i8) {
        let _ = apply(|v| v as Field, x);
    }";
    let error = get_monomorphized(src).expect_err("a signed source is never cast to Field");
    let diagnostic = CustomDiagnostic::from(error);
    assert!(
        diagnostic.message.contains("Only unsigned integer types may be casted to Field"),
        "{}",
        diagnostic.message
    );
}

#[test]
fn comptime_modulus_builtins_describe_the_configured_field() {
    for field in FieldId::ALL {
        let config = FieldConfig::new(field);
        let le_bytes = config.modulus().to_bytes_le();
        let last = le_bytes.len() - 1;
        let src = format!(
            "{}
        fn main() {{
            comptime {{
                assert(modulus_num_bits() == {});
                let le_bytes = modulus_le_bytes();
                assert(le_bytes[0] == {});
                assert(le_bytes[{last}] == {});
                let be_bits = modulus_be_bits();
                assert(be_bits[0]);
            }}
        }}",
            stdlib_src::MODULUS,
            config.num_bits(),
            le_bytes[0],
            le_bytes[last],
        );
        let options =
            GetProgramOptions { root_and_stdlib: true, ..GetProgramOptions::for_field(field) };
        let errors = get_program_with_options(&src, options).2;
        assert!(errors.is_empty(), "{field}: {errors:?}");
    }
}

#[test]
fn comptime_field_arithmetic_wraps_under_the_configured_field() {
    for field in FieldId::ALL {
        let largest = FieldConfig::new(field).modulus() - 1u8;
        let src = format!(
            "fn main() {{
                comptime {{
                    let largest: Field = {largest};
                    assert(largest + 1 == 0);
                    assert(0 - 1 == largest);
                    assert(largest * largest == 1);
                    assert(2 * (1 / 2) == 1);
                    assert(-largest == 1);
                }}
            }}"
        );
        let errors = get_program_errors_for_field(&src, field);
        assert!(errors.is_empty(), "{field}: {errors:?}");
    }
}

#[test]
fn a_comptime_field_value_reaches_the_program_in_the_configured_field() {
    let largest = FieldConfig::new(FieldId::Goldilocks).modulus() - 1u8;
    let src = format!("fn main() {{ let _x: Field = comptime {{ {largest} + 1 }}; }}");

    for field in FieldId::ALL {
        let result =
            if field == FieldId::Goldilocks { BigUint::ZERO } else { largest.clone() + 1u8 };
        let program = get_monomorphized_for_field(&src, field).unwrap().to_string();
        assert_eq!(
            program.trim(),
            format!("fn main$f0() -> () {{\n    let _x$l0 = {result}\n}}"),
            "{field}"
        );
    }
}

#[test]
fn type_level_field_arithmetic_wraps_under_the_configured_field() {
    for field in FieldId::ALL {
        let largest = FieldConfig::new(field).modulus() - 1u8;
        let src = format!(
            "fn value<let N: Field>() -> Field {{ N }}
             fn main() {{ let _ = value::<{largest}_Field + 1_Field>(); }}"
        );
        let program = get_monomorphized_for_field(&src, field).unwrap().to_string();
        assert!(program.contains("fn value$f1() -> Field {\n    0\n}"), "{field}: {program}");
    }
}

#[test]
fn type_level_integer_arithmetic_works_under_every_field() {
    let src = "fn value<let N: u64>() -> u64 { N }
               fn main() { assert(value::<18446744069414584320_u64 + 1_u64>() != 0); }";
    for field in FieldId::ALL {
        let program = get_monomorphized_for_field(src, field).unwrap().to_string();
        assert!(program.contains("18446744069414584321"), "{field}: {program}");
    }
}

#[test]
fn comptime_crypto_uses_the_configured_field() {
    let programs = [
        ("poseidon2_permutation", "
            #[foreign(poseidon2_permutation)]
            fn permute(input: [Field; 4]) -> [Field; 4] {}
            fn main() { comptime { let result = permute([0, 1, 2, 3]);
                assert(result[0] == 0x01bd538c2ee014ed5141b29e9ae240bf8db3fe5b9a38629a9647cf8d76c01737);
            } }
        "),
        ("derive_pedersen_generators", "
            struct Point { x: Field, y: Field }
            #[builtin(derive_pedersen_generators)]
            fn generators(domain: [u8; 1], start: u32) -> [Point; 1] {}
            fn main() { comptime { let point = generators([0], 0)[0];
                assert(point.x != 0);
                assert(point.y * point.y == point.x * point.x * point.x - 17);
            } }
        "),
    ];
    for (builtin, source) in programs {
        for field in FieldId::ALL {
            let options =
                GetProgramOptions { root_and_stdlib: true, ..GetProgramOptions::for_field(field) };
            let errors = get_program_with_options(source, options).2;
            if field == FieldId::Bn254 {
                assert!(errors.is_empty(), "{builtin}: {errors:?}");
            } else {
                assert!(errors.iter().any(|error| matches!(error,
                    CompilationError::InterpreterError(crate::hir::comptime::InterpreterError::Unimplemented { item, .. })
                        if item.starts_with(builtin)
                )), "{field}, {builtin}: {errors:?}");
            }
        }
    }
}

/// The reason the first entry point type is refused, looking past the aliases and struct fields
/// that contain it.
fn entry_point_refusal(errors: &[CompilationError]) -> Option<InvalidType> {
    errors.iter().find_map(|error| match error {
        CompilationError::TypeError(TypeCheckError::InvalidTypeForEntryPoint {
            invalid_type,
            ..
        }) => Some(invalid_type.innermost().clone()),
        _ => None,
    })
}

/// An entry point's values cross as bytes, the same on every field, so it takes and returns
/// integers of every legal width, wherever its type holds them.
#[test]
fn every_legal_integer_width_crosses_main() {
    for field in FieldId::ALL {
        for typ in ["u2", "u8", "u24", "u34", "u64", "u128", "u253", "u16384", "i2", "i64", "i66"] {
            for src in [
                format!("fn main(x: {typ}) {{ assert(x == x); }}"),
                format!("fn main(x: pub {typ}) {{ assert(x == x); }}"),
                format!("fn main() -> pub {typ} {{ 0 }}"),
                format!("fn main(x: [{typ}; 2]) {{ assert(x[0] == x[1]); }}"),
                format!("fn main(x: (bool, {typ})) {{ assert(x.1 == x.1); }}"),
                format!(
                    "struct Pair {{ a: bool, b: {typ} }}
                     fn main(x: Pair) {{ assert(x.b == x.b); }}"
                ),
                format!(
                    "type Word = {typ};
                     fn main(x: Word) {{ assert(x == x); }}"
                ),
            ] {
                let errors = get_program_errors_for_field(&src, field);
                assert!(errors.is_empty(), "{field}: `{src}`: {errors:?}");
            }
        }
        for src in [
            "struct Wrapper<let N: u32> { inner: u<N> }
             fn main(x: Wrapper<64>) { assert(x.inner == x.inner); }",
            "fn main(x: u<2 * 32>) { assert(x == x); }",
        ] {
            let errors = get_program_errors_for_field(src, field);
            assert!(errors.is_empty(), "{field}: `{src}`: {errors:?}");
        }
    }
}

/// `Field` crosses no entry point, wherever the type holds it.
#[test]
fn field_is_refused_at_main_wherever_the_type_holds_it() {
    let programs = [
        "fn main(x: Field) { assert(x == x); }",
        "fn main(x: pub Field) { assert(x == x); }",
        "fn main() -> pub Field { 0 }",
        "fn main(x: [Field; 2]) { assert(x[0] == x[1]); }",
        "fn main(x: (u8, Field)) { assert(x.0 == x.0); }",
        "struct Pair { a: u8, b: Field }
         fn main(x: Pair) { assert(x.a == x.a); }",
        "struct Inner { value: Field }
         struct Outer { inner: [Inner; 2] }
         fn main(x: Outer) { assert(x.inner[0].value == x.inner[1].value); }",
        "type Word = Field;
         fn main(x: Word) { assert(x == x); }",
        "struct Wrapper<T> { inner: T }
         fn main(x: Wrapper<Field>) { assert(x.inner == x.inner); }",
        "fn main() -> pub (u8, [Field; 1]) { (0, [0]) }",
    ];
    for field in FieldId::ALL {
        for src in programs {
            let errors = get_program_errors_for_field(src, field);
            assert_eq!(errors.len(), 1, "{field}: `{src}`: {errors:?}");
            assert!(
                matches!(entry_point_refusal(&errors), Some(InvalidType::Field(_))),
                "{field}: `{src}`: {errors:?}"
            );
        }
    }
}

/// Only `main` and contract functions are entry points: helpers, test and fuzz functions keep
/// `Field`, as do values computed inside the program.
#[test]
fn the_entry_point_rule_leaves_other_functions_alone() {
    let src = "
        fn helper(x: Field, y: u24) -> Field { assert(y == y); x + 1 }

        #[test]
        fn tested(x: Field, y: u24) { assert(x == x); assert(y == y); }

        #[fuzz]
        fn fuzzed(x: Field, y: u24) { assert(x == x); assert(y == y); }

        fn main(x: u32, z: bool) -> pub u32 {
            let y = helper(2, x as u24);
            assert(y == y);
            assert(z == z);
            x
        }
    ";
    for field in FieldId::ALL {
        let errors = get_program_errors_for_field(src, field);
        assert!(errors.is_empty(), "{field}: {errors:?}");
    }
}

/// A contract function is an entry point too; a library method in the contract is not.
#[test]
fn contract_functions_follow_the_entry_point_rule() {
    let src = "
        contract Wallet {
            pub fn balance(x: u24) -> pub u64 { x as u64 }

            #[contract_library_method]
            pub fn double(x: Field) -> Field { x * 2 }
        }
    ";
    for field in FieldId::ALL {
        let errors = get_program_errors_for_field(src, field);
        assert!(errors.is_empty(), "{field}: {errors:?}");
    }

    let src = "
        contract Wallet {
            pub fn balance(x: Field) -> pub Field { x }
        }
    ";
    for field in FieldId::ALL {
        let errors = get_program_errors_for_field(src, field);
        assert_eq!(errors.len(), 2, "{field}: {errors:?}");
        for error in &errors {
            assert!(
                matches!(
                    entry_point_refusal(std::slice::from_ref(error)),
                    Some(InvalidType::Field(_))
                ),
                "{field}: {errors:?}"
            );
        }
    }
}

/// An `#[abi(tag)]` global is not an input of `main`, so it keeps `Field`.
#[test]
fn abi_tag_globals_keep_field() {
    let src = "
        contract Wallet {
            #[abi(tag)]
            global LIMIT: (Field, u24) = (1, 2);
        }
    ";
    for field in FieldId::ALL {
        let errors = get_program_errors_for_field(src, field);
        assert!(errors.is_empty(), "{field}: {errors:?}");
    }
}

#[test]
fn the_entry_point_diagnostic_explains_the_field_rule() {
    let src = "
    struct Pair {
           ~~~~ Struct Pair has an invalid entry point type
        a: u8,
        b: Field,
        ~ Field b has an invalid entry point type
        ~ Field is not a valid entry point type. Found: Field
    }

    fn main(x: Pair) {
               ^^^^ Invalid type found in the entry point to a program
               ~~~~ This type has an invalid entry point type inside it
        assert(x.a == x.a);
    }
    ";
    let options = GetProgramOptions { allow_elaborator_errors: true, ..Default::default() };
    check_errors_with_options(src, false, options);

    let errors =
        get_program_errors_for_field("fn main(x: Field) { assert(x == x); }", FieldId::Bn254);
    let diagnostic = CustomDiagnostic::from(&errors[0]);
    assert_eq!(
        diagnostic.notes,
        vec![
            "Note: main and contract functions take and return no `Field`, so that a program's inputs are read the same way on every field. Use an integer type, or bytes such as `[u8; 32]` for a hash or a curve coordinate, and convert inside the program."
                .to_string()
        ]
    );
}

/// A struct marked `#[field(..)]` is field-specific, so it crosses no entry point, directly, in a
/// struct field or as the return, even though the gate admits the field the tests compile for
/// (bn254).
#[test]
fn a_field_gated_struct_is_refused_at_main() {
    let src = "
    #[field(bn254)]
    ~~~~~~~~~~~~~~~ Made field-specific here
    struct Gated { x: u32 }

    struct Outer { inner: Gated }
           ~~~~~ Struct Outer has an invalid entry point type
                   ~~~~~ Field inner has an invalid entry point type
                   ~~~~~ A field-specific type is not a valid entry point type. Found: Gated

    fn main(x: Gated, y: Outer) -> pub Gated {
               ^^^^^ Invalid type found in the entry point to a program
               ~~~~~ A field-specific type is not a valid entry point type. Found: Gated
                         ^^^^^ Invalid type found in the entry point to a program
                         ~~~~~ This type has an invalid entry point type inside it
                                       ^^^^^ Invalid type found in the entry point to a program
                                       ~~~~~ A field-specific type is not a valid entry point type. Found: Gated
        assert(x.x == y.inner.x);
        x
    }
    ";
    check_errors(src);
}

#[test]
fn a_field_gated_alias_is_refused_at_main() {
    let src = "
    #[field(bn254)]
    ~~~~~~~~~~~~~~~ Made field-specific here
    type Word = u32;

    fn main(_x: Word) {}
                ^^^^ Invalid type found in the entry point to a program
                ~~~~ A field-specific type is not a valid entry point type. Found: Word
    ";
    check_errors(src);
}

/// A type declared in a module marked `#[field(..)]` is field-specific through that module.
#[test]
fn a_type_in_a_field_gated_module_is_refused_at_main() {
    let src = "
    #[field(bn254)]
    ~~~~~~~~~~~~~~~ Made field-specific here
    mod gated {
        pub struct Inner { pub x: u32 }
    }

    fn main(_x: gated::Inner) {}
                ^^^^^^^^^^^^ Invalid type found in the entry point to a program
                ~~~~~~~~~~~~ A field-specific type is not a valid entry point type. Found: Inner
    ";
    check_errors(src);
}

/// Field-specific items stay usable inside the program and by test functions; only the entry
/// point signature refuses them.
#[test]
fn field_gated_items_are_allowed_off_the_entry_point_signature() {
    let src = "
    #[field(bn254)]
    struct Gated { x: u32 }

    #[field(bn254)]
    fn helper(g: Gated) -> u32 { g.x }

    #[test]
    fn tested(g: Gated) { assert(helper(g) == g.x); }

    fn main(x: u32) -> pub u32 {
        helper(Gated { x })
    }
    ";
    assert_no_errors(src);
}

/// A program runs on every field, so neither `main` nor a contract with entry points may be
/// marked `#[field(..)]`, on the function itself or on the enclosing contract.
#[test]
fn the_field_attribute_is_refused_on_entry_points() {
    let main = "
    #[field(bn254)]
    ^^^^^^^^^^^^^^^ entry point function main cannot be field-specific
    ~~~~~~~~~~~~~~~ #[field] attribute not allowed on an entry point
    fn main() {}
    ";
    check_errors(main);

    let contract_function = "
    contract Wallet {
        #[field(bn254)]
        ^^^^^^^^^^^^^^^ entry point function balance cannot be field-specific
        ~~~~~~~~~~~~~~~ #[field] attribute not allowed on an entry point
        pub fn balance(x: u32) -> pub u32 { x }
    }
    ";
    check_errors(contract_function);

    let contract = "
    #[field(bn254)]
    ^^^^^^^^^^^^^^^ entry point function balance cannot be field-specific
    ~~~~~~~~~~~~~~~ #[field] attribute not allowed on an entry point
    contract Wallet {
        pub fn balance(x: u32) -> pub u32 { x }
    }
    ";
    check_errors(contract);
}

/// A gate is found through enclosing modules, aliases and generic instantiations, under every
/// field, including the one it selects.
#[test]
fn field_gates_are_found_through_modules_aliases_and_generics() {
    for field in FieldId::ALL {
        for declarations in [
            "#[field(FIELD)] mod outer { pub mod inner { pub struct Gated { pub x: u32 } } }
             type Gated = outer::inner::Gated;",
            "#[field(FIELD)] mod outer { pub mod inner { pub type Word = u32; } }
             type Gated = outer::inner::Word;",
        ] {
            let src = format!(
                "{} struct Wrapper<T> {{ value: T }}
                 fn main(_x: [Wrapper<(bool, Gated)>; 1]) {{}}",
                declarations.replace("FIELD", field.name())
            );
            let errors = get_program_errors_for_field(&src, field);
            assert_eq!(errors.len(), 1, "{field}: `{src}`: {errors:?}");
            assert!(
                matches!(entry_point_refusal(&errors), Some(InvalidType::FieldGated { .. })),
                "{field}: `{src}`: {errors:?}"
            );
        }
    }
}

/// Unit's no-return meaning does not erase a field-specific alias from the written signature.
#[test]
fn a_unit_return_still_checks_field_gates_on_its_aliases() {
    for field in FieldId::ALL {
        let source = format!(
            "#[field({})] type Nothing = (); type Other = Nothing;
             fn main() -> pub Other {{}}",
            field.name()
        );
        let errors = get_program_errors_for_field(&source, field);
        assert_eq!(errors.len(), 1, "{field}: {errors:?}");
        assert!(
            matches!(entry_point_refusal(&errors), Some(InvalidType::FieldGated { .. })),
            "{field}: {errors:?}"
        );
    }
}

/// `Expr::as_integer` reads a literal into a `Field`, which must be canonical in the configured
/// field: a literal at or above the modulus is an error, never "not an integer".
#[test]
fn expr_as_integer_refuses_a_literal_at_or_above_the_modulus() {
    for field in FieldId::ALL {
        let modulus = FieldConfig::new(field).modulus();
        let src = format!(
            "
            struct Option<T> {{ _is_some: bool, _value: T }}
            impl Quoted {{
                #[builtin(quoted_as_expr)]
                comptime fn as_expr(self) -> Option<Expr> {{}}
            }}
            impl Expr {{
                #[builtin(expr_as_integer)]
                comptime fn as_integer(self) -> Option<Field> {{}}
            }}
            fn main() {{
                comptime {{
                    let largest = quote {{ {} }}.as_expr()._value.as_integer();
                    assert(largest._is_some);
                    assert(largest._value == -1);
                    let _ = quote {{ {modulus} }}.as_expr()._value.as_integer();
                }}
            }}",
            modulus - 1u8
        );
        let options =
            GetProgramOptions { root_and_stdlib: true, ..GetProgramOptions::for_field(field) };
        let errors = get_program_with_options(&src, options).2;
        assert!(
            errors.iter().any(|error| matches!(
                error,
                CompilationError::InterpreterError(
                    crate::hir::comptime::InterpreterError::IntegerOutOfRangeForType { value, .. }
                ) if value == &num_bigint::BigInt::from(modulus.clone())
            )),
            "{field}: {errors:?}"
        );
    }
}
