//! Field rules across configurations, with comptime execution restricted to the linked field.

use acvm::{FieldConfig, FieldId};
use noirc_errors::CustomDiagnostic;

use crate::hir::def_collector::dc_crate::CompilationError;
use crate::hir::type_check::TypeCheckError;
use crate::test_utils::{
    GetProgramOptions, get_monomorphized, get_monomorphized_for_field, get_program_with_options,
    stdlib_src,
};
use crate::tests::{assert_no_errors, get_program_errors_for_field};

#[test]
fn field_literals_must_be_canonical() {
    for field in FieldId::ALL {
        let modulus = FieldConfig::new(field).modulus();

        let largest = format!("fn main() {{ let _: Field = {}; }}", modulus - 1u8);
        let errors = get_program_errors_for_field(&largest, field);
        assert!(errors.is_empty(), "{field}: {errors:?}");

        let too_large = format!("fn main() {{ let _: Field = {modulus}; }}");
        let errors = get_program_errors_for_field(&too_large, field);
        assert!(
            errors.iter().any(|error| matches!(
                error,
                CompilationError::TypeError(TypeCheckError::IntegerLiteralDoesNotFitItsType { .. })
            )),
            "{field}: expected the literal `{modulus}` to be rejected, got {errors:?}"
        );
    }
}

#[test]
fn integer_literals_above_the_modulus_stay_exact() {
    for field in FieldId::ALL {
        let src = format!("fn main() -> pub u64 {{ {} }}", u64::MAX);
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
        if accepted && field != FieldId::linked() {
            continue;
        }
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

    fn main(x: u64) -> pub Field {
        apply(|v| v as Field, x)
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

    fn main(x: i8) -> pub Field {
        apply(|v| v as Field, x)
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
fn comptime_modulus_builtins_describe_the_linked_field() {
    let field = FieldId::linked();
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

// TODO: Remove this rejection test once comptime evaluation supports the configured field.
#[test]
fn comptime_evaluation_rejects_a_field_other_than_the_linked_field() {
    let src = "fn main() -> pub u64 { comptime { let x: Field = 0; (x - 1) as u64 } }";
    for field in FieldId::ALL.into_iter().filter(|field| *field != FieldId::linked()) {
        let errors = get_program_errors_for_field(src, field);
        assert!(
            errors.iter().any(|error| matches!(
                error,
                CompilationError::InterpreterError(crate::hir::comptime::InterpreterError::Unimplemented { item, .. })
                    if item == &format!("Comptime evaluation for {field} in a compiler built for {}", FieldId::linked())
            )),
            "{field}: {errors:?}"
        );
    }
}

#[test]
fn type_level_field_evaluation_rejects_a_field_other_than_the_linked_field() {
    for field in FieldId::ALL.into_iter().filter(|field| *field != FieldId::linked()) {
        let modulus = FieldConfig::new(field).modulus();
        for expression in [
            format!("{}_Field + 1_Field", modulus - 1u8),
            format!("{modulus}_Field"),
            "-1_Field".to_owned(),
        ] {
            let src = format!(
                "fn value<let N: Field>() -> Field {{ N }}
                 fn main() -> pub Field {{ value::<{expression}>() }}"
            );
            let errors = get_program_errors_for_field(&src, field);
            assert!(
                errors.iter().any(|error| matches!(
                    error,
                    CompilationError::InterpreterError(crate::hir::comptime::InterpreterError::Unimplemented { item, .. })
                        if item == &format!("Type-level Field evaluation for {field} in a compiler built for {}", FieldId::linked())
                )),
                "{field}, {expression}: {errors:?}"
            );
        }
    }
}

#[test]
fn type_level_field_arithmetic_wraps_under_the_linked_field() {
    let largest = FieldConfig::linked().modulus() - 1u8;
    let src = format!(
        "fn value<let N: Field>() -> Field {{ N }}
         fn main() -> pub Field {{ value::<{largest}_Field + 1_Field>() }}"
    );
    let program = get_monomorphized(&src).unwrap().to_string();
    assert!(program.contains("fn value$f1() -> Field {\n    0\n}"), "{program}");
}

#[test]
fn type_level_integer_arithmetic_works_under_every_field() {
    let src = "fn value<let N: u64>() -> u64 { N }
               fn main() -> pub u64 { value::<18446744069414584320_u64 + 1_u64>() }";
    for field in FieldId::ALL {
        let program = get_monomorphized_for_field(src, field).unwrap().to_string();
        assert!(program.contains("18446744069414584321"), "{field}: {program}");
    }
}
